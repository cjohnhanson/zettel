use camino::{Utf8Path, Utf8PathBuf};

use crate::config::ZettelConfig;
use crate::error::{Error, Result};
use crate::note::{self, Note, NoteFrontmatter};

use mdstore::slug::{extract_prefix, generate_prefix, slugify};

#[derive(Default)]
pub struct CreateNoteOptions {
    pub tags: Option<String>,
    pub links: Option<String>,
    pub body: Option<String>,
}

#[derive(Default)]
pub struct EditNoteOptions<'a> {
    pub title: Option<&'a str>,
    pub tags: Option<&'a str>,
    pub add_tag: Option<&'a str>,
    pub remove_tag: Option<&'a str>,
    pub links: Option<&'a str>,
    pub add_link: Option<&'a str>,
    pub remove_link: Option<&'a str>,
    pub body: Option<&'a str>,
    pub append: Option<&'a str>,
}

/// A backlink: a note that references the target.
#[derive(Debug, Serialize)]
pub struct Backlink {
    pub id: String,
    pub title: String,
}

use serde::Serialize;

pub struct Repo {
    pub root: Utf8PathBuf,
    pub config: ZettelConfig,
}

impl Repo {
    pub fn open(root: &Utf8Path) -> Result<Self> {
        let config_path = root.join("zettel.yml");
        if !config_path.exists() {
            return Err(Error::NotInitialized);
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: ZettelConfig = serde_yml::from_str(&content)?;
        Ok(Repo {
            root: root.to_owned(),
            config,
        })
    }

    pub fn zettel_dir(&self) -> Utf8PathBuf {
        self.root.join(&self.config.zettel_dir)
    }

    // -- Init --

    pub fn init(root: &Utf8Path) -> Result<()> {
        let config_path = root.join("zettel.yml");
        if config_path.exists() {
            return Err(Error::AlreadyInitialized);
        }
        std::fs::write(&config_path, "zettel_dir: .zettel\n")?;
        std::fs::create_dir_all(root.join(".zettel"))?;
        Ok(())
    }

    // -- ID resolution --

    pub fn resolve_id(&self, input: &str) -> Result<String> {
        let dir = self.zettel_dir();

        // Exact match
        if dir.join(format!("{input}.md")).exists() {
            return Ok(input.to_string());
        }

        // 4-char prefix match
        if input.len() == 4
            && input
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            let prefix_dash = format!("{input}-");
            let mut matches = Vec::new();
            Self::scan_prefix_matches(&dir, &prefix_dash, &mut matches)?;
            match matches.len() {
                0 => {}
                1 => return Ok(matches.into_iter().next().unwrap()),
                _ => return Err(Error::AmbiguousPrefix(input.into())),
            }
        }

        // Slug match
        let mut slug_matches = Vec::new();
        Self::scan_slug_matches(&dir, input, &mut slug_matches)?;
        if slug_matches.len() == 1 {
            return Ok(slug_matches.into_iter().next().unwrap());
        }

        Err(Error::NoteNotFound(input.into()))
    }

    fn scan_prefix_matches(dir: &Utf8Path, prefix_dash: &str, out: &mut Vec<String>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md") {
                let stem = path.file_stem().unwrap_or("");
                if stem.starts_with(prefix_dash) && !out.contains(&stem.to_string()) {
                    out.push(stem.to_string());
                }
            }
        }
        Ok(())
    }

    fn scan_slug_matches(dir: &Utf8Path, slug: &str, out: &mut Vec<String>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md") {
                let stem = path.file_stem().unwrap_or("");
                if let Some((_, file_slug)) = extract_prefix(stem)
                    && file_slug == slug
                    && !out.contains(&stem.to_string())
                {
                    out.push(stem.to_string());
                }
            }
        }
        Ok(())
    }

    fn collect_existing_prefixes(&self) -> Result<Vec<String>> {
        let dir = self.zettel_dir();
        let mut prefixes = Vec::new();
        if !dir.exists() {
            return Ok(prefixes);
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md") {
                let stem = path.file_stem().unwrap_or("");
                if let Some((prefix, _)) = extract_prefix(stem)
                    && !prefixes.iter().any(|p: &String| p == prefix)
                {
                    prefixes.push(prefix.to_string());
                }
            }
        }
        Ok(prefixes)
    }

    fn slug_exists(&self, slug: &str) -> Result<bool> {
        let dir = self.zettel_dir();
        if !dir.exists() {
            return Ok(false);
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md") {
                let stem = path.file_stem().unwrap_or("");
                if let Some((_, file_slug)) = extract_prefix(stem) {
                    if file_slug == slug {
                        return Ok(true);
                    }
                } else if stem == slug {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    // -- Notes --

    pub fn create_note(&self, title: &str, opts: CreateNoteOptions) -> Result<String> {
        let slug = slugify(title);

        if self.slug_exists(&slug)? {
            return Err(Error::NoteAlreadyExists(slug));
        }

        let existing_prefixes = self.collect_existing_prefixes()?;
        let prefix = generate_prefix(&existing_prefixes);
        let id = format!("{prefix}-{slug}");

        let dir = self.zettel_dir();
        std::fs::create_dir_all(&dir)?;
        let note_path = dir.join(format!("{id}.md"));

        let mut fm = note::new_frontmatter(title);
        if let Some(t) = opts.tags {
            fm.tags = t.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Some(l) = opts.links {
            fm.links = l.split(',').map(|s| s.trim().to_string()).collect();
        }

        let body = opts.body.as_deref().unwrap_or("");
        let content = note::serialize_note(&fm, body);
        std::fs::write(&note_path, content)?;

        Ok(id)
    }

    pub fn list_notes(&self, tag_filter: Option<&str>) -> Result<Vec<Note>> {
        let dir = self.zettel_dir();
        let mut notes = Vec::new();

        if !dir.exists() {
            return Ok(notes);
        }

        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md") {
                let id = path.file_stem().unwrap_or("").to_string();
                let content = std::fs::read_to_string(&path)?;
                let (fm, body) = note::parse_note(&content)?;
                notes.push(Note {
                    id,
                    frontmatter: fm,
                    body,
                });
            }
        }

        if let Some(tag) = tag_filter {
            notes.retain(|n| n.frontmatter.tags.iter().any(|t| t == tag));
        }

        notes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(notes)
    }

    pub fn find_note(&self, id: &str) -> Result<Note> {
        let resolved = self.resolve_id(id)?;
        let dir = self.zettel_dir();
        let path = dir.join(format!("{resolved}.md"));

        if !path.exists() {
            return Err(Error::NoteNotFound(id.into()));
        }

        let content = std::fs::read_to_string(&path)?;
        let (fm, body) = note::parse_note(&content)?;
        Ok(Note {
            id: resolved,
            frontmatter: fm,
            body,
        })
    }

    pub fn edit_note(&self, id: &str, opts: EditNoteOptions<'_>) -> Result<()> {
        let n = self.find_note(id)?;
        let dir = self.zettel_dir();
        let note_path = dir.join(format!("{}.md", n.id));
        let content = std::fs::read_to_string(&note_path)?;
        let (mut fm, mut body) = note::parse_note(&content)?;

        if let Some(new_title) = opts.title {
            fm.title = new_title.to_string();
        }

        if let Some(new_body) = opts.body {
            body = new_body.to_string();
        }

        if let Some(append_text) = opts.append {
            if body.is_empty() {
                body = append_text.to_string();
            } else {
                body.push_str("\n\n");
                body.push_str(append_text);
            }
        }

        if let Some(t) = opts.tags {
            fm.tags = t.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Some(tag) = opts.add_tag
            && !fm.tags.iter().any(|t| t == tag)
        {
            fm.tags.push(tag.to_string());
        }

        if let Some(tag) = opts.remove_tag {
            fm.tags.retain(|t| t != tag);
        }

        if let Some(l) = opts.links {
            fm.links = l.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Some(link) = opts.add_link
            && !fm.links.iter().any(|l| l == link)
        {
            fm.links.push(link.to_string());
        }

        if let Some(link) = opts.remove_link {
            fm.links.retain(|l| l != link);
        }

        note::update_timestamp(&mut fm);
        let new_content = note::serialize_note(&fm, &body);
        std::fs::write(&note_path, new_content)?;
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<()> {
        let resolved = self.resolve_id(id)?;
        let dir = self.zettel_dir();
        let path = dir.join(format!("{resolved}.md"));
        if !path.exists() {
            return Err(Error::NoteNotFound(id.into()));
        }
        std::fs::remove_file(&path)?;
        Ok(())
    }

    // -- Links & backlinks --

    /// Find all notes that link to the given note (by ID or prefix).
    pub fn backlinks(&self, id: &str) -> Result<Vec<Backlink>> {
        let resolved = self.resolve_id(id)?;
        let prefix = extract_prefix(&resolved).map(|(p, _)| p.to_string());

        let all_notes = self.list_notes(None)?;
        let mut results = Vec::new();

        for n in &all_notes {
            if n.id == resolved {
                continue;
            }

            let links_to_target = n.frontmatter.links.iter().any(|link| {
                link == &resolved
                    || prefix
                        .as_ref()
                        .is_some_and(|p| link == p)
            });

            // Also check for [[id]] references in the body
            let body_links = body_contains_link(&n.body, &resolved, prefix.as_deref());

            if links_to_target || body_links {
                results.push(Backlink {
                    id: n.id.clone(),
                    title: n.frontmatter.title.clone(),
                });
            }
        }

        Ok(results)
    }

    /// Find notes that have no incoming or outgoing links.
    pub fn orphans(&self) -> Result<Vec<Note>> {
        let all_notes = self.list_notes(None)?;
        let all_ids: Vec<&str> = all_notes.iter().map(|n| n.id.as_str()).collect();

        let mut orphans = Vec::new();
        for n in &all_notes {
            let has_outgoing = !n.frontmatter.links.is_empty();
            let has_incoming = all_notes.iter().any(|other| {
                other.id != n.id
                    && (other.frontmatter.links.iter().any(|l| {
                        l == &n.id
                            || extract_prefix(&n.id)
                                .map(|(p, _)| p)
                                .is_some_and(|p| l == p)
                    }) || body_contains_link(
                        &other.body,
                        &n.id,
                        extract_prefix(&n.id).map(|(p, _)| p),
                    ))
            });
            let has_body_outgoing = all_ids
                .iter()
                .any(|other_id| *other_id != n.id && body_contains_link(&n.body, other_id, extract_prefix(other_id).map(|(p, _)| p)));

            if !has_outgoing && !has_incoming && !has_body_outgoing {
                orphans.push(Note {
                    id: n.id.clone(),
                    frontmatter: NoteFrontmatter {
                        title: n.frontmatter.title.clone(),
                        tags: n.frontmatter.tags.clone(),
                        links: n.frontmatter.links.clone(),
                        created: n.frontmatter.created.clone(),
                        updated: n.frontmatter.updated.clone(),
                    },
                    body: n.body.clone(),
                });
            }
        }

        Ok(orphans)
    }
}

/// Check if a markdown body contains a `[[ref]]` link to the given ID.
fn body_contains_link(body: &str, full_id: &str, prefix: Option<&str>) -> bool {
    let full_ref = format!("[[{full_id}]]");
    if body.contains(&full_ref) {
        return true;
    }
    if let Some(p) = prefix {
        let prefix_ref = format!("[[{p}]]");
        if body.contains(&prefix_ref) {
            return true;
        }
    }
    false
}

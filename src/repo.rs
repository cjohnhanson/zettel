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
    /// The default provenance spec for the note.
    pub provenance: Option<String>,
}

#[derive(Default)]
pub struct ListNotesFilter<'a> {
    pub tag: Option<&'a str>,
    /// Comma-separated provenance filter tokens. A note matches when any
    /// of its spans matches any token.
    pub provenance: Option<&'a str>,
    /// Keep only the notes with at least one unreviewed agent span.
    pub unreviewed: bool,
}

#[derive(Default)]
pub struct EditNoteOptions<'a> {
    pub title: Option<&'a str>,
    pub provenance: Option<&'a str>,
    pub tags: Option<&'a str>,
    pub add_tag: Option<&'a str>,
    pub remove_tag: Option<&'a str>,
    pub links: Option<&'a str>,
    pub add_link: Option<&'a str>,
    pub remove_link: Option<&'a str>,
    pub body: Option<&'a str>,
    pub append: Option<&'a str>,
}

/// A backlink. This is a note that references the target note.
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
        let config: ZettelConfig = yaml_serde::from_str(&content)?;
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

        // 4-character prefix match
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
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
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
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
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
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
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
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
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
        if let Some(spec) = opts.provenance {
            crate::provenance::parse_authored_spec(&spec)?;
            fm.provenance = Some(spec);
        }
        if let Some(t) = opts.tags {
            fm.tags = t.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Some(l) = opts.links {
            fm.links = l.split(',').map(|s| s.trim().to_string()).collect();
        }

        let body = opts.body.as_deref().unwrap_or("");
        // A marker the vocabulary does not accept makes every span in
        // the note read as unknown, and the note can then never be
        // reviewed. Refuse it at the point of writing.
        crate::provenance::resolve_spans(fm.provenance.as_deref(), body)?;
        let content = note::serialize_note(&fm, body);
        mdstore::store::write_document(note_path.as_std_path(), &content)
            .map_err(crate::provenance::from_mdstore)?;

        Ok(id)
    }

    pub fn list_notes(&self, filter: &ListNotesFilter<'_>) -> Result<Vec<Note>> {
        let dir = self.zettel_dir();
        let mut notes = Vec::new();

        if !dir.exists() {
            return Ok(notes);
        }

        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
                let id = path.file_stem().unwrap_or("").to_string();
                let content = mdstore::store::read_document(path.as_std_path())
                    .map_err(crate::provenance::from_mdstore)?;
                // One unparseable file must not take down a repo-wide
                // command. Skip it with a warning; `zettel check` names it.
                match note::parse_note(&content) {
                    Ok((fm, body)) => notes.push(Note {
                        id,
                        frontmatter: fm,
                        body,
                    }),
                    Err(e) => eprintln!("warning: skipping {id}: {e}"),
                }
            }
        }

        if let Some(tag) = filter.tag {
            notes.retain(|n| n.frontmatter.tags.iter().any(|t| t == tag));
        }

        if let Some(tokens) = filter.provenance {
            let tokens: Vec<&str> = tokens.split(',').map(str::trim).collect();
            for token in &tokens {
                crate::provenance::validate_filter_token(token)?;
            }
            notes.retain(|n| {
                crate::provenance::note_matches_tokens(
                    n.frontmatter.provenance.as_deref(),
                    &n.body,
                    &tokens,
                )
            });
        }

        if filter.unreviewed {
            notes.retain(|n| {
                crate::provenance::note_has_unreviewed(n.frontmatter.provenance.as_deref(), &n.body)
            });
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

        let content = mdstore::store::read_document(path.as_std_path())
            .map_err(crate::provenance::from_mdstore)?;
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
        let content = mdstore::store::read_document(note_path.as_std_path())
            .map_err(crate::provenance::from_mdstore)?;
        let (mut fm, mut body) = note::parse_note(&content)?;

        if let Some(new_title) = opts.title {
            fm.title = new_title.to_string();
        }

        if let Some(spec) = opts.provenance {
            let mut new_marker = crate::provenance::parse_authored_spec(spec)?;
            // The same origin and qualifier keep the existing attrs (a
            // review stamp survives a no-op re-set); a different origin
            // or qualifier drops them, because the review no longer
            // applies to what the spec now claims.
            if new_marker.attrs.is_empty()
                && let Some(old) = fm
                    .provenance
                    .as_deref()
                    .and_then(|s| crate::provenance::parse_spec(s).ok())
                && old.origin == new_marker.origin
                && old.qualifier == new_marker.qualifier
            {
                new_marker.attrs = old.attrs;
            }
            fm.provenance = Some(new_marker.to_string());
        }

        let body_changed = opts.body.is_some() || opts.append.is_some();

        if let Some(new_body) = opts.body {
            body = new_body.to_string();
        }

        if let Some(append_text) = opts.append {
            if body.is_empty() {
                body = append_text.to_string();
            } else {
                // A body ending inside an open span would absorb the
                // appended text and mislabel it. Close the span first.
                if mdstore::provenance::ends_open(&body).unwrap_or(false) {
                    body.push_str("\n<!-- /prov -->");
                }
                body.push_str("\n\n");
                body.push_str(append_text);
            }
        }

        // Changed content invalidates the default's review stamp: the
        // stamp vouched for text that is no longer there.
        if body_changed
            && let Some(spec) = fm.provenance.as_deref()
            && let Ok(mut d) = crate::provenance::parse_spec(spec)
            && d.attr(crate::provenance::ATTR_REVIEWED).is_some()
        {
            d.attrs.retain(|(k, _)| {
                k != crate::provenance::ATTR_REVIEWED && k != crate::provenance::ATTR_REVIEWER
            });
            fm.provenance = Some(d.to_string());
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

        crate::provenance::resolve_spans(fm.provenance.as_deref(), &body)?;
        note::update_timestamp(&mut fm);
        let new_content = note::serialize_note(&fm, &body);
        mdstore::store::write_document(note_path.as_std_path(), &new_content)
            .map_err(crate::provenance::from_mdstore)?;
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

    /// The note IDs a note cites through citation-origin spans, resolved.
    /// A source that does not resolve is an external key, not an error.
    fn resolved_citations(&self, n: &Note) -> Vec<String> {
        crate::provenance::citation_refs(n.frontmatter.provenance.as_deref(), &n.body)
            .iter()
            .filter_map(|r| self.resolve_id(r).ok())
            .collect()
    }

    /// Find all notes that link to the given note. The link is a full ID or a
    /// prefix.
    pub fn backlinks(&self, id: &str) -> Result<Vec<Backlink>> {
        let resolved = self.resolve_id(id)?;
        let prefix = extract_prefix(&resolved).map(|(p, _)| p.to_string());

        let all_notes = self.list_notes(&ListNotesFilter::default())?;
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
                    || self.resolve_id(link).ok().as_deref() == Some(&resolved)
            });

            // Also check the body for [[id]] references
            let body_links = body_contains_link(&n.body, &resolved, prefix.as_deref());

            // A citation of a note is a graph edge too.
            let cites_target = self.resolved_citations(n).iter().any(|c| c == &resolved);

            if links_to_target || body_links || cites_target {
                results.push(Backlink {
                    id: n.id.clone(),
                    title: n.frontmatter.title.clone(),
                });
            }
        }

        Ok(results)
    }

    /// Find the notes with no incoming links and no outgoing links.
    pub fn orphans(&self) -> Result<Vec<Note>> {
        let all_notes = self.list_notes(&ListNotesFilter::default())?;
        let all_ids: Vec<&str> = all_notes.iter().map(|n| n.id.as_str()).collect();

        let mut orphans = Vec::new();
        for n in &all_notes {
            let has_outgoing =
                !n.frontmatter.links.is_empty() || !self.resolved_citations(n).is_empty();
            let has_incoming = all_notes.iter().any(|other| {
                other.id != n.id
                    && (other.frontmatter.links.iter().any(|l| {
                        l == &n.id
                            || extract_prefix(&n.id)
                                .map(|(p, _)| p)
                                .is_some_and(|p| l == p)
                            || self.resolve_id(l).ok().as_deref() == Some(n.id.as_str())
                    }) || body_contains_link(
                        &other.body,
                        &n.id,
                        extract_prefix(&n.id).map(|(p, _)| p),
                    ) || self.resolved_citations(other).contains(&n.id))
            });
            let has_body_outgoing = all_ids
                .iter()
                .any(|other_id| *other_id != n.id && body_contains_link(&n.body, other_id, extract_prefix(other_id).map(|(p, _)| p)));

            if !has_outgoing && !has_incoming && !has_body_outgoing {
                orphans.push(Note {
                    id: n.id.clone(),
                    frontmatter: NoteFrontmatter {
                        title: n.frontmatter.title.clone(),
                        provenance: n.frontmatter.provenance.clone(),
                        tags: n.frontmatter.tags.clone(),
                        links: n.frontmatter.links.clone(),
                        created: n.frontmatter.created.clone(),
                        updated: n.frontmatter.updated.clone(),
                        extra: n.frontmatter.extra.clone(),
                    },
                    body: n.body.clone(),
                });
            }
        }

        Ok(orphans)
    }

    // -- Search --

    pub fn search(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let re = regex::Regex::new(pattern)
            .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;

        let all_notes = self.list_notes(&ListNotesFilter::default())?;
        let mut results = Vec::new();

        for n in all_notes {
            let mut matched_fields = Vec::new();
            if re.is_match(&n.frontmatter.title) {
                matched_fields.push("title".to_string());
            }
            if n.frontmatter.tags.iter().any(|t| re.is_match(t)) {
                matched_fields.push("tags".to_string());
            }
            if re.is_match(&n.body) {
                matched_fields.push("body".to_string());
            }
            if !matched_fields.is_empty() {
                results.push(SearchResult {
                    note: n,
                    matched_fields,
                });
            }
        }

        Ok(results)
    }

    // -- Context (the neighborhood of a note) --

    pub fn context(&self, id: &str, depth: usize) -> Result<Vec<Note>> {
        let resolved = self.resolve_id(id)?;
        let all_notes = self.list_notes(&ListNotesFilter::default())?;

        let mut collected: Vec<String> = vec![resolved.clone()];
        let mut frontier: Vec<String> = vec![resolved];

        for _ in 0..depth {
            let mut next_frontier = Vec::new();
            for current_id in &frontier {
                // Forward links from this note
                if let Some(n) = all_notes.iter().find(|n| &n.id == current_id) {
                    for link in &n.frontmatter.links {
                        if let Ok(resolved_link) = self.resolve_id(link)
                            && !collected.contains(&resolved_link)
                        {
                            collected.push(resolved_link.clone());
                            next_frontier.push(resolved_link);
                        }
                    }
                    // A citation of a note is a forward edge too.
                    for cited in self.resolved_citations(n) {
                        if !collected.contains(&cited) {
                            collected.push(cited.clone());
                            next_frontier.push(cited);
                        }
                    }
                    // Also check the body for [[ref]] links
                    for other in &all_notes {
                        if !collected.contains(&other.id)
                            && body_contains_link(&n.body, &other.id, extract_prefix(&other.id).map(|(p, _)| p))
                        {
                            collected.push(other.id.clone());
                            next_frontier.push(other.id.clone());
                        }
                    }
                }

                // Backlinks to this note
                for other in &all_notes {
                    if collected.contains(&other.id) {
                        continue;
                    }
                    let links_here = other.frontmatter.links.iter().any(|l| {
                        l == current_id
                            || self.resolve_id(l).ok().as_deref() == Some(current_id.as_str())
                    }) || body_contains_link(
                        &other.body,
                        current_id,
                        extract_prefix(current_id).map(|(p, _)| p),
                    ) || self
                        .resolved_citations(other)
                        .iter()
                        .any(|c| c == current_id);
                    if links_here {
                        collected.push(other.id.clone());
                        next_frontier.push(other.id.clone());
                    }
                }
            }
            frontier = next_frontier;
            if frontier.is_empty() {
                break;
            }
        }

        // Return the full notes in the collected order
        let mut result = Vec::new();
        for id in &collected {
            if let Some(n) = all_notes.iter().find(|n| &n.id == id) {
                result.push(Note {
                    id: n.id.clone(),
                    frontmatter: NoteFrontmatter {
                        title: n.frontmatter.title.clone(),
                        provenance: n.frontmatter.provenance.clone(),
                        tags: n.frontmatter.tags.clone(),
                        links: n.frontmatter.links.clone(),
                        created: n.frontmatter.created.clone(),
                        updated: n.frontmatter.updated.clone(),
                        extra: n.frontmatter.extra.clone(),
                    },
                    body: n.body.clone(),
                });
            }
        }

        Ok(result)
    }

    // -- Stats --

    pub fn stats(&self) -> Result<Stats> {
        let all_notes = self.list_notes(&ListNotesFilter::default())?;

        let total = all_notes.len();

        // Span counts by origin, across all notes
        let mut span_counts = SpanCounts::default();
        for n in &all_notes {
            let spans = crate::provenance::resolve_spans_lenient(
                n.frontmatter.provenance.as_deref(),
                &n.body,
            );
            for s in &spans {
                match s.marker.as_ref().map(|m| m.origin.as_str()) {
                    Some(crate::provenance::ORIGIN_HUMAN) => span_counts.human += 1,
                    Some(crate::provenance::ORIGIN_AGENT) => span_counts.agent += 1,
                    Some(crate::provenance::ORIGIN_CITATION) => span_counts.citation += 1,
                    _ => span_counts.unknown += 1,
                }
                if crate::provenance::is_unreviewed_agent(s.marker.as_ref()) {
                    span_counts.unreviewed_agent += 1;
                }
            }
        }

        // Tag frequency
        let mut tag_counts: Vec<(String, usize)> = Vec::new();
        for n in &all_notes {
            for tag in &n.frontmatter.tags {
                if let Some(entry) = tag_counts.iter_mut().find(|(t, _)| t == tag) {
                    entry.1 += 1;
                } else {
                    tag_counts.push((tag.clone(), 1));
                }
            }
        }
        tag_counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        // The most connected notes, by backlink count
        let mut backlink_counts: Vec<(String, String, usize)> = Vec::new();
        for n in &all_notes {
            let count = self.backlinks(&n.id)?.len();
            backlink_counts.push((n.id.clone(), n.frontmatter.title.clone(), count));
        }
        backlink_counts.sort_by_key(|(_, _, count)| std::cmp::Reverse(*count));

        let orphan_count = self.orphans()?.len();

        Ok(Stats {
            total,
            span_counts,
            tag_counts,
            most_connected: backlink_counts.into_iter().take(5).collect(),
            orphan_count,
        })
    }

    // -- Check (broken links) --

    pub fn check(&self) -> Result<Vec<BrokenLink>> {
        let mut broken = Vec::new();

        // Name the files that list_notes skips: the diagnostic command
        // must survive the corruption it diagnoses.
        let dir = self.zettel_dir();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = Utf8PathBuf::try_from(entry.path())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
                    let content = mdstore::store::read_document(path.as_std_path())
                    .map_err(crate::provenance::from_mdstore)?;
                    if let Err(e) = note::parse_note(&content) {
                        broken.push(BrokenLink {
                            source_id: path.file_stem().unwrap_or("").to_string(),
                            source_title: "unparseable".to_string(),
                            target: e.to_string(),
                            location: "frontmatter".to_string(),
                        });
                    }
                }
            }
        }

        let all_notes = self.list_notes(&ListNotesFilter::default())?;

        // Link resolution lives in the workspace, which knows the store
        // closure; checking it here too would report a working
        // cross-store link as broken.
        for n in &all_notes {
            // Check the provenance vocabulary. The repo-wide commands
            // degrade a bad note to unknown; this is where it gets named.
            if let Err(e) =
                crate::provenance::resolve_spans(n.frontmatter.provenance.as_deref(), &n.body)
            {
                broken.push(BrokenLink {
                    source_id: n.id.clone(),
                    source_title: n.frontmatter.title.clone(),
                    target: e.to_string(),
                    location: "provenance".to_string(),
                });
            }
        }

        Ok(broken)
    }

    // -- Provenance review --

    /// The spans of a note, resolved against its default provenance.
    /// Strict: an invalid marker is an error here, unlike the repo-wide
    /// commands that degrade a bad note to unknown.
    pub fn note_spans(&self, id: &str) -> Result<(Note, Vec<crate::provenance::ResolvedSpan>)> {
        let n = self.find_note(id)?;
        let spans =
            crate::provenance::resolve_spans(n.frontmatter.provenance.as_deref(), &n.body)?;
        Ok((n, spans))
    }

    /// Stamp `reviewed=<today>` on agent spans. `spans` holds 1-based
    /// indices as `note review` shows them; `None` stamps every unreviewed
    /// agent span. A stamp on an unmarked span goes to the note's default
    /// provenance spec, so it covers all unmarked text; the default counts
    /// once no matter how many unmarked spans it covers. A note with no
    /// body spans approves through its default alone. Returns the number
    /// of spans stamped.
    pub fn approve_spans(
        &self,
        id: &str,
        spans: Option<&[usize]>,
        reviewer: Option<&str>,
    ) -> Result<usize> {
        let n = self.find_note(id)?;
        let note_path = self.zettel_dir().join(format!("{}.md", n.id));
        let content = mdstore::store::read_document(note_path.as_std_path())
            .map_err(crate::provenance::from_mdstore)?;
        let (mut fm, body) = note::parse_note(&content)?;

        let mut raw = mdstore::provenance::parse_spans(&body)
            .map_err(crate::provenance::from_mdstore)?;
        // The review command validates like the listing does: an invalid
        // marker never gets approved.
        for span in &raw {
            if let Some(m) = &span.marker {
                crate::provenance::validate_marker(m)?;
            }
        }
        let mut default = fm
            .provenance
            .as_deref()
            .map(crate::provenance::parse_spec)
            .transpose()?;

        let today = format!("{}", chrono::Utc::now().format("%Y-%m-%d"));
        let stamp_default = |d: &mut mdstore::Marker| {
            d.set_attr(crate::provenance::ATTR_REVIEWED, &today);
            if let Some(r) = reviewer {
                d.set_attr(crate::provenance::ATTR_REVIEWER, r);
            }
        };

        let mut targets: Vec<usize> = match spans {
            Some(list) => {
                for &i in list {
                    if i == 0 || i > raw.len() {
                        return Err(Error::SpanNotFound(i));
                    }
                    let span = &raw[i - 1];
                    // A separator never shows in the review listing, so a
                    // number the user never saw is "not found", not a
                    // silent default approval.
                    if crate::provenance::is_separator(span.marker.as_ref(), &span.text) {
                        return Err(Error::SpanNotFound(i));
                    }
                    let effective = span.marker.as_ref().or(default.as_ref());
                    let agent = effective
                        .is_some_and(|m| m.origin == crate::provenance::ORIGIN_AGENT);
                    if !agent {
                        return Err(Error::SpanNotAgent(i));
                    }
                }
                list.iter().map(|&i| i - 1).collect()
            }
            None => (0..raw.len())
                .filter(|&i| {
                    if crate::provenance::is_separator(raw[i].marker.as_ref(), &raw[i].text) {
                        return false;
                    }
                    let effective = raw[i].marker.as_ref().or(default.as_ref());
                    crate::provenance::is_unreviewed_agent(effective)
                })
                .collect(),
        };
        // A duplicated index stamps and counts once.
        let mut seen: Vec<usize> = Vec::new();
        targets.retain(|i| {
            if seen.contains(i) {
                false
            } else {
                seen.push(*i);
                true
            }
        });

        // An empty body has no spans; `--approve all` stamps the default.
        if raw.is_empty()
            && spans.is_none()
            && crate::provenance::is_unreviewed_agent(default.as_ref())
        {
            let d = default.as_mut().expect("unreviewed agent default exists");
            stamp_default(d);
            fm.provenance = Some(d.to_string());
            note::update_timestamp(&mut fm);
            mdstore::store::write_document(
                note_path.as_std_path(),
                &note::serialize_note(&fm, &body),
            )
            .map_err(crate::provenance::from_mdstore)?;
            return Ok(1);
        }

        let mut stamped = 0usize;
        let mut default_stamped = false;
        for i in targets {
            match &mut raw[i].marker {
                Some(m) => {
                    m.set_attr(crate::provenance::ATTR_REVIEWED, &today);
                    if let Some(r) = reviewer {
                        m.set_attr(crate::provenance::ATTR_REVIEWER, r);
                    }
                    stamped += 1;
                }
                None => {
                    // An unmarked span carries the note default, which
                    // covers every other unmarked span too. Stamping
                    // the default would approve text the reviewer did
                    // not name, so a named span gets its own marker
                    // holding exactly what the reviewer approved.
                    if spans.is_some() {
                        if let Some(d) = &default {
                            let mut own = d.clone();
                            own.set_attr(crate::provenance::ATTR_REVIEWED, &today);
                            if let Some(r) = reviewer {
                                own.set_attr(crate::provenance::ATTR_REVIEWER, r);
                            }
                            raw[i].marker = Some(own);
                            stamped += 1;
                        }
                    } else if let Some(d) = &mut default
                        && !default_stamped
                    {
                        // 'all' approves everything the default covers,
                        // so stamping the default once is exact.
                        stamp_default(d);
                        default_stamped = true;
                        stamped += 1;
                    }
                }
            }
        }

        if stamped > 0 {
            if let Some(d) = &default {
                fm.provenance = Some(d.to_string());
            }
            let new_body = mdstore::provenance::render_spans(&raw);
            note::update_timestamp(&mut fm);
            mdstore::store::write_document(
                note_path.as_std_path(),
                &note::serialize_note(&fm, &new_body),
            )
            .map_err(crate::provenance::from_mdstore)?;
        }
        Ok(stamped)
    }

    // -- Migration --

    /// Convert pre-provenance notes. `status: permanent` becomes
    /// `provenance: human` (a permanent note was rewritten by its author);
    /// the status key is removed either way. A second run changes nothing.
    pub fn migrate(&self) -> Result<Vec<(String, MigrateAction)>> {
        let dir = self.zettel_dir();
        let mut changes = Vec::new();
        if !dir.exists() {
            return Ok(changes);
        }
        let status_key = yaml_serde::Value::String("status".into());
        let mut paths: Vec<Utf8PathBuf> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if path.extension() == Some("md")
                && mdstore::store::is_regular_file(path.as_std_path())
            {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            let id = path.file_stem().unwrap_or("").to_string();
            let content = mdstore::store::read_document(path.as_std_path())
            .map_err(crate::provenance::from_mdstore)?;
            let (mut fm, body) = note::parse_note(&content)?;
            let Some(status) = fm.extra.remove(&status_key) else {
                continue;
            };
            let action = if status.as_str() == Some("permanent") && fm.provenance.is_none() {
                fm.provenance = Some("human".to_string());
                MigrateAction::SetHuman
            } else {
                MigrateAction::RemovedStatus
            };
            mdstore::store::write_document(
                path.as_std_path(),
                &note::serialize_note(&fm, &body),
            )
            .map_err(crate::provenance::from_mdstore)?;
            changes.push((id, action));
        }
        Ok(changes)
    }
}

/// What `migrate` did to one note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MigrateAction {
    /// `status: permanent` became `provenance: human`.
    SetHuman,
    /// The status key was removed; the provenance stays as it was.
    RemovedStatus,
}

impl std::fmt::Display for MigrateAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetHuman => f.write_str("provenance: human"),
            Self::RemovedStatus => f.write_str("status key removed"),
        }
    }
}

/// One problem that `check` reports.
///
/// Not every problem is a broken link. Calling an unreachable store or
/// a skipped file a broken link sent a reader looking for a reference
/// that does not exist.
#[derive(Debug, Serialize)]
pub struct Finding {
    /// What holds the problem: a note ID, or a config file.
    pub source_id: String,
    pub source_title: String,
    /// What is wrong.
    pub target: String,
    /// The class of problem, so a caller can group them.
    pub location: String,
}

/// The old name, kept so a caller that used it still compiles.
pub type BrokenLink = Finding;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub note: Note,
    pub matched_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total: usize,
    pub span_counts: SpanCounts,
    pub tag_counts: Vec<(String, usize)>,
    pub most_connected: Vec<(String, String, usize)>,
    pub orphan_count: usize,
}

/// Body span counts by origin, across the whole knowledge base.
#[derive(Debug, Default, Serialize)]
pub struct SpanCounts {
    pub human: usize,
    pub agent: usize,
    pub citation: usize,
    pub unknown: usize,
    /// Agent spans with no `reviewed=` attribute.
    pub unreviewed_agent: usize,
}

/// Extract all `[[ref]]` references from a markdown body.
pub(crate) fn extract_body_refs(body: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut remaining = body;
    while let Some(start) = remaining.find("[[") {
        let after = &remaining[start + 2..];
        if let Some(end) = after.find("]]") {
            let reference = &after[..end];
            if !reference.is_empty() && !refs.contains(&reference.to_string()) {
                refs.push(reference.to_string());
            }
            remaining = &after[end + 2..];
        } else {
            break;
        }
    }
    refs
}

/// Return true if a markdown body contains a `[[ref]]` link to the given ID.
/// The reference matches the full ID, the 4-character prefix, or the slug.
fn body_contains_link(body: &str, full_id: &str, prefix: Option<&str>) -> bool {
    // [[full-id]]
    let full_ref = format!("[[{full_id}]]");
    if body.contains(&full_ref) {
        return true;
    }
    // [[prefix]]
    if let Some(p) = prefix {
        let prefix_ref = format!("[[{p}]]");
        if body.contains(&prefix_ref) {
            return true;
        }
    }
    // [[slug]], the part after the prefix
    if let Some((_, slug)) = extract_prefix(full_id) {
        let slug_ref = format!("[[{slug}]]");
        if body.contains(&slug_ref) {
            return true;
        }
    }
    false
}

/// One note prepared for reading: its identity, its frontmatter
/// summary, and the spans an interface should show.
///
/// `read` returns this so the CLI and a server present the same
/// content. Only the rendering differs.
#[derive(Debug, Serialize)]
pub struct ReadNote {
    pub id: String,
    pub title: String,
    pub provenance: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    /// The spans to show. With no filter this holds one span carrying
    /// the whole body, so a reader never has to reassemble it.
    pub spans: Vec<ReadSpan>,
}

/// One span of a note that a read returns.
#[derive(Debug, Serialize)]
pub struct ReadSpan {
    /// The resolved provenance, or `unknown`.
    pub provenance: String,
    /// True when the span carries the whole unfiltered body.
    pub whole_body: bool,
    pub text: String,
}

impl Repo {
    /// Read the matching notes, with the spans a provenance filter
    /// selects.
    ///
    /// With no filter, each note gives one span holding its whole body,
    /// markers included. With a filter, each note gives only the spans
    /// that match, and a note with no matching span is left out.
    pub fn read(&self, tag: Option<&str>, provenance: Option<&str>) -> Result<Vec<ReadNote>> {
        let notes = self.list_notes(&ListNotesFilter {
            tag,
            provenance,
            unreviewed: false,
        })?;
        let tokens: Option<Vec<&str>> = provenance.map(|t| t.split(',').map(str::trim).collect());

        let mut out = Vec::new();
        for n in notes {
            let spans = match &tokens {
                None => {
                    let mut spans = Vec::new();
                    if !n.body.is_empty() {
                        spans.push(ReadSpan {
                            provenance: n
                                .frontmatter
                                .provenance
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                            whole_body: true,
                            text: n.body.clone(),
                        });
                    }
                    spans
                }
                Some(tokens) => crate::provenance::resolve_spans_lenient(
                    n.frontmatter.provenance.as_deref(),
                    &n.body,
                )
                .iter()
                .filter(|s| crate::provenance::matches_any(s.marker.as_ref(), tokens))
                .map(|s| ReadSpan {
                    provenance: crate::provenance::display(s.marker.as_ref()),
                    whole_body: false,
                    text: s.text.clone(),
                })
                .collect(),
            };
            out.push(ReadNote {
                id: n.id,
                title: n.frontmatter.title,
                provenance: n
                    .frontmatter
                    .provenance
                    .unwrap_or_else(|| "unknown".to_string()),
                tags: n.frontmatter.tags,
                links: n.frontmatter.links,
                spans,
            });
        }
        Ok(out)
    }
}

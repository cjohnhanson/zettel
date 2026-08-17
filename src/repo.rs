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
    /// The note directory, already checked against the store root.
    ///
    /// The loader guarded this and the Repo did not, so the two layers
    /// disagreed about which directory this store holds. A
    /// `zettel_dir` that climbed out then let the CLI read and write
    /// another store's notes under bare, unqualified ids, while the
    /// commands built on the loader reported an empty store.
    notes_dir: Utf8PathBuf,
    /// The authority to read and write inside the note directory, and
    /// nowhere else.
    ///
    /// A checked path is a check every caller must remember. A handle
    /// is a check none of them can skip: the operating system refuses
    /// a name that leaves the directory, whoever built it.
    ///
    /// Opened on first use, not in `open`. Opening eagerly made every
    /// command fail on a store whose note directory is absent, and
    /// creating the directory to avoid that put a mkdir on a read
    /// command. A read has no business making one, and through a
    /// symlinked note directory the mkdir landed outside the store.
    ///
    /// One handle, held: once opened it is reused, so a root swapped
    /// afterwards cannot redirect a later call. The containment check
    /// moves with the open rather than staying in `open`, because a
    /// check that runs long before the open guards nothing.
    notes: std::sync::OnceLock<mdstore::confined::StoreDir>,
}

impl Repo {
    pub fn open(root: &Utf8Path) -> Result<Self> {
        let config_path = root.join("zettel.yml");
        if !config_path.exists() {
            return Err(Error::NotInitialized);
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: ZettelConfig = yaml_serde::from_str(&content)?;
        // Resolve the directory once, here, through the one function
        // that decides containment. An accessor that joined the raw
        // configured value gave every caller an unguarded path.
        let notes_dir = mdstore::store::document_dir(root.as_std_path(), &config.zettel_dir)
            .map_err(crate::provenance::from_mdstore)?;
        let notes_dir = Utf8PathBuf::try_from(notes_dir)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        // A store whose note directory does not exist yet is an empty
        // store, not a broken one. git tracks no empty directory, so a
        // clone made before the first note has none, and opening the
        // handle here made list, check and stats all fail where they
        // used to answer.
        Ok(Repo {
            root: root.to_owned(),
            config,
            notes_dir,
            notes: std::sync::OnceLock::new(),
        })
    }

    /// The handle, opened on first use.
    ///
    /// A caller that reads gets an error when the directory is absent;
    /// a caller that writes creates it first through `notes_mut`.
    ///
    /// Containment is decided here, not only in `open`. Deferring the
    /// open defers the check with it: a note directory replaced by a
    /// link between `open` and the first read would otherwise be
    /// followed, and the window is the whole of `Workspace::open`,
    /// which walks every dependency store.
    fn notes(&self) -> Result<&mdstore::confined::StoreDir> {
        if let Some(notes) = self.notes.get() {
            return Ok(notes);
        }
        let checked =
            mdstore::store::document_dir(self.root.as_std_path(), &self.config.zettel_dir)
                .map_err(crate::provenance::from_mdstore)?;
        if checked != self.notes_dir.as_std_path() {
            return Err(Error::NoteNotFound(self.notes_dir.to_string()));
        }
        let notes = mdstore::confined::StoreDir::open(self.notes_dir.as_std_path())
            .map_err(crate::provenance::from_mdstore)?;
        Ok(self.notes.get_or_init(|| notes))
    }

    /// The handle, creating the note directory if it is absent.
    ///
    /// Only a write calls this. A store with no note directory yet is
    /// an empty store to every reader, and becomes a real one the
    /// first time something is written.
    fn notes_mut(&self) -> Result<&mdstore::confined::StoreDir> {
        if !self.notes_dir.exists() {
            std::fs::create_dir_all(&self.notes_dir)?;
        }
        self.notes()
    }

    /// Read one note through the handle.
    ///
    /// The caller passes the path it already built, and the name is
    /// taken back off it against the note directory. The strip is
    /// bookkeeping, not a guard: Utf8Path::join is lexical, so a path
    /// built from an escaping id strips cleanly back to the escaping
    /// name. The handle is what refuses it.
    fn read_note_file(&self, path: &Utf8Path) -> Result<String> {
        let rel = self.relative(path);
        self.notes()?
            .read(&rel)
            .map_err(crate::provenance::from_mdstore)
    }

    /// Write one note through the handle.
    fn write_note_file(&self, path: &Utf8Path, contents: &str) -> Result<()> {
        let rel = self.relative(path);
        // A write creates the note directory when it is absent, so a
        // store becomes real on its first note rather than on its
        // first read.
        self.notes_mut()?
            .write(&rel, contents)
            .map_err(crate::provenance::from_mdstore)
    }

    /// The name of a path inside the note directory.
    ///
    /// Every caller builds its path by joining onto the note
    /// directory, so the strip always succeeds. A path from anywhere
    /// else is passed through whole and the handle refuses it, which
    /// is the same answer by a shorter road. Returning a Result here
    /// invited a NoteNotFound for what would be a containment failure.
    fn relative(&self, path: &Utf8Path) -> String {
        path.strip_prefix(&self.notes_dir).map_or_else(
            |_| path.as_str().to_string(),
            |rel| rel.as_str().to_string(),
        )
    }

    /// Every note stem in this store, through the handle.
    fn note_stems(&self) -> Result<Vec<String>> {
        // A store with no note directory yet holds no notes. That is
        // an empty store, not a failure.
        //
        // Only absence is swallowed. A directory that exists and
        // cannot be opened is a fault: a mode-000 note directory
        // reported an empty list with a success status, so a person
        // whose notes vanished got no signal at all.
        if !self.notes_dir.exists() {
            return Ok(Vec::new());
        }
        let notes = self.notes()?;
        let scan = notes.scan("").map_err(crate::provenance::from_mdstore)?;
        Ok(scan.entries.into_iter().map(|e| e.stem).collect())
    }

    pub fn zettel_dir(&self) -> Utf8PathBuf {
        self.notes_dir.clone()
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
        // An id becomes a file path here, so it must name one document
        // and not a path. Without this, `note edit ../../outside` and
        // `note delete /etc/anything` reach outside the store.
        if !mdstore::is_plain_stem(input) {
            return Err(Error::NoteNotFound(input.to_string()));
        }

        // Exact match. Through the handle, because Path::exists
        // follows a link: a link planted at <id>.md resolved to an id
        // that list and scan both refuse to show.
        if self
            .notes()
            .is_ok_and(|n| n.is_document(&format!("{input}.md")))
        {
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
            self.scan_prefix_matches(&prefix_dash, &mut matches)?;
            match matches.len() {
                0 => {}
                1 => return Ok(matches.into_iter().next().unwrap()),
                _ => return Err(Error::AmbiguousPrefix(input.into())),
            }
        }

        // Slug match
        let mut slug_matches = Vec::new();
        self.scan_slug_matches(input, &mut slug_matches)?;
        if slug_matches.len() == 1 {
            return Ok(slug_matches.into_iter().next().unwrap());
        }

        Err(Error::NoteNotFound(input.into()))
    }

    /// Note stems whose id begins with this prefix.
    ///
    /// The handle lists the directory, so a link planted among the
    /// notes is skipped by type rather than resolved.
    fn scan_prefix_matches(&self, prefix_dash: &str, out: &mut Vec<String>) -> Result<()> {
        for stem in self.note_stems()? {
            if stem.starts_with(prefix_dash) && !out.contains(&stem) {
                out.push(stem);
            }
        }
        Ok(())
    }

    /// Note stems whose slug matches, whatever their prefix.
    fn scan_slug_matches(&self, slug: &str, out: &mut Vec<String>) -> Result<()> {
        for stem in self.note_stems()? {
            if let Some((_, file_slug)) = extract_prefix(&stem)
                && file_slug == slug
                && !out.contains(&stem)
            {
                out.push(stem);
            }
        }
        Ok(())
    }

    fn collect_existing_prefixes(&self) -> Result<Vec<String>> {
        let mut prefixes = Vec::new();
        for stem in self.note_stems()? {
            if let Some((prefix, _)) = extract_prefix(&stem)
                && !prefixes.iter().any(|p: &String| p == prefix)
            {
                prefixes.push(prefix.to_string());
            }
        }
        Ok(prefixes)
    }

    fn slug_exists(&self, slug: &str) -> Result<bool> {
        for stem in self.note_stems()? {
            {
                let stem = stem.as_str();
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
        crate::provenance::refuse_authored_stamps(body)?;
        let content = note::serialize_note(&fm, body);
        self.write_note_file(&note_path, &content)?;

        Ok(id)
    }

    pub fn list_notes(&self, filter: &ListNotesFilter<'_>) -> Result<Vec<Note>> {
        let dir = self.zettel_dir();
        let mut notes = Vec::new();

        for stem in self.note_stems()? {
            {
                let path = dir.join(format!("{stem}.md"));
                let id = stem;
                // One unreadable file must not take down a repo-wide
                // command either. A single non-UTF-8 byte made list,
                // search and read fail for the whole store.
                let content = match self.read_note_file(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("warning: skipping {id}: {e}");
                        continue;
                    }
                };
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

        // No exists() probe here. resolve_id already answered through
        // the handle, so a second test adds nothing, and Path::exists
        // follows a link, which is the predicate this whole surface
        // replaced.

        let content = self.read_note_file(&path)?;
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
        let content = self.read_note_file(&note_path)?;
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

        // Guard the caller's text, never the merged body: the body on
        // disk may already hold stamps that an approval wrote.
        if let Some(text) = opts.body {
            crate::provenance::refuse_authored_stamps(text)?;
        }
        if let Some(text) = opts.append {
            crate::provenance::refuse_authored_stamps(text)?;
        }

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
        self.write_note_file(&note_path, &new_content)?;
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<()> {
        let resolved = self.resolve_id(id)?;
        let name = format!("{resolved}.md");
        if !self.notes().is_ok_and(|n| n.is_document(&name)) {
            return Err(Error::NoteNotFound(id.into()));
        }
        // A delete goes through the handle for the same reason a read
        // does. An id that is secretly a path cannot name a file
        // outside the note directory.
        self.notes()?
            .remove(&name)
            .map_err(crate::provenance::from_mdstore)
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

    // -- Search --

    /// The longest pattern a caller may send.
    ///
    /// A served store takes this text from the network, and a long
    /// pattern is the raw material of an expensive one.
    const MAX_PATTERN_BYTES: usize = 1024;

    /// The compiled size a pattern may reach, in bytes.
    ///
    /// The default is 10MB, which accepts patterns whose matching cost
    /// grows to minutes over a store of any size. A bounded repetition
    /// such as `[\s\S]{3000}zzz` compiles well under the default and
    /// then runs for hours. A low limit refuses it at compile time,
    /// where the cost is nothing.
    const PATTERN_SIZE_LIMIT: usize = 64 * 1024;

    pub fn search(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        if pattern.len() > Self::MAX_PATTERN_BYTES {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "the pattern is {} bytes; the limit is {}",
                    pattern.len(),
                    Self::MAX_PATTERN_BYTES
                ),
            )));
        }
        let re = regex::RegexBuilder::new(pattern)
            .size_limit(Self::PATTERN_SIZE_LIMIT)
            .dfa_size_limit(Self::PATTERN_SIZE_LIMIT)
            .build()
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
                            && body_contains_link(
                                &n.body,
                                &other.id,
                                extract_prefix(&other.id).map(|(p, _)| p),
                            )
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

    // -- Check (broken links) --

    // -- Provenance review --

    /// The spans of a note, resolved against its default provenance.
    /// Strict: an invalid marker is an error here, unlike the repo-wide
    /// commands that degrade a bad note to unknown.
    pub fn note_spans(&self, id: &str) -> Result<(Note, Vec<crate::provenance::ResolvedSpan>)> {
        let n = self.find_note(id)?;
        let spans = crate::provenance::resolve_spans(n.frontmatter.provenance.as_deref(), &n.body)?;
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
        let content = self.read_note_file(&note_path)?;
        let (mut fm, body) = note::parse_note(&content)?;

        let mut raw =
            mdstore::provenance::parse_spans(&body).map_err(crate::provenance::from_mdstore)?;
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
                    let agent =
                        effective.is_some_and(|m| m.origin == crate::provenance::ORIGIN_AGENT);
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
            self.write_note_file(&note_path, &note::serialize_note(&fm, &body))?;
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
            self.write_note_file(&note_path, &note::serialize_note(&fm, &new_body))?;
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
        let status_key = yaml_serde::Value::String("status".into());
        let mut paths: Vec<Utf8PathBuf> = self
            .note_stems()?
            .into_iter()
            .map(|stem| dir.join(format!("{stem}.md")))
            .collect();
        paths.sort();
        for path in paths {
            let id = path.file_stem().unwrap_or("").to_string();
            let content = self.read_note_file(&path)?;
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
            self.write_note_file(&path, &note::serialize_note(&fm, &body))?;
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
    /// Orphans across the whole closure, which is what the graph
    /// computes. The note count above covers this store only, so the
    /// two are different scopes and the output says which.
    pub orphan_count: usize,
    /// Orphans that live in this store.
    pub local_orphan_count: usize,
    /// True when the closure holds more than this store.
    pub has_dependencies: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A note id is caller text. It must not name a file outside the
    /// note directory, whatever it spells.
    #[test]
    fn a_note_id_cannot_name_a_file_outside_the_store() {
        let base = std::env::temp_dir().join(format!("zettel-escape-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("store/.zettel")).unwrap();
        std::fs::write(base.join("store/zettel.yml"), "zettel_dir: .zettel\n").unwrap();
        std::fs::write(base.join("secret.md"), "SECRET").unwrap();

        let root = Utf8PathBuf::try_from(base.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();

        let outside = repo.zettel_dir().join("../../secret.md");
        assert!(
            repo.read_note_file(&outside).is_err(),
            "a climbing note path was read"
        );
        assert!(
            repo.write_note_file(&outside, "overwritten").is_err(),
            "a climbing note path was written"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("secret.md")).unwrap(),
            "SECRET",
            "the file outside the store changed"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// A store whose note directory does not exist yet is empty, not
    /// broken. git tracks no empty directory, so a clone made before
    /// the first note has none.
    #[test]
    fn a_store_with_no_note_directory_yet_still_opens() {
        let base = fixture("absent");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("zettel.yml"), "zettel_dir: notes\n").unwrap();
        let root = Utf8PathBuf::try_from(base.clone()).unwrap();

        let repo = Repo::open(&root).expect("a store with no notes yet must open");
        assert!(
            repo.list_notes(&ListNotesFilter::default())
                .unwrap()
                .is_empty()
        );
        repo.create_note("First", CreateNoteOptions::default())
            .expect("the first note must still be creatable");
        assert_eq!(
            repo.list_notes(&ListNotesFilter::default()).unwrap().len(),
            1
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A read must not create the note directory.
    ///
    /// Creating it eagerly put a mkdir on every command. Through a
    /// symlinked note directory that mkdir landed outside the store,
    /// because the containment check is skipped while the joined path
    /// does not exist yet. A read that creates nothing cannot.
    #[test]
    fn a_read_does_not_create_the_note_directory() {
        let base = fixture("noread");
        std::fs::create_dir_all(base.join("store")).unwrap();
        std::fs::create_dir_all(base.join("outside")).unwrap();
        std::fs::write(base.join("store/zettel.yml"), "zettel_dir: link/notes\n").unwrap();
        std::os::unix::fs::symlink(base.join("outside"), base.join("store/link")).unwrap();

        let root = Utf8PathBuf::try_from(base.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();
        assert!(
            repo.list_notes(&ListNotesFilter::default())
                .unwrap()
                .is_empty()
        );
        assert!(
            !base.join("outside/notes").exists(),
            "a read created a directory outside the store"
        );

        // A plain store, no link. Still no directory from a read.
        let plain = fixture("noread2");
        std::fs::create_dir_all(plain.join("store")).unwrap();
        std::fs::write(plain.join("store/zettel.yml"), "zettel_dir: notes\n").unwrap();
        let root = Utf8PathBuf::try_from(plain.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();
        assert!(
            repo.list_notes(&ListNotesFilter::default())
                .unwrap()
                .is_empty()
        );
        assert!(
            !plain.join("store/notes").exists(),
            "a read created the note directory"
        );

        // A write creates it, which is where a store becomes real.
        repo.create_note("First", CreateNoteOptions::default())
            .unwrap();
        assert!(plain.join("store/notes").is_dir());
        assert_eq!(
            repo.list_notes(&ListNotesFilter::default()).unwrap().len(),
            1
        );

        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&plain);
    }

    /// Deferring the open defers the containment check with it.
    ///
    /// Repo::open decided containment, then the handle opened at an
    /// arbitrary later time. A note directory replaced by a link in
    /// between was followed, and the window is the whole of
    /// Workspace::open, which walks every dependency store.
    #[test]
    fn a_note_directory_swapped_after_open_is_not_followed() {
        let base = fixture("swapafter");
        std::fs::create_dir_all(base.join("store/.zettel")).unwrap();
        std::fs::create_dir_all(base.join("elsewhere")).unwrap();
        std::fs::write(base.join("store/zettel.yml"), "zettel_dir: .zettel\n").unwrap();
        std::fs::write(
            base.join("store/.zettel/aaaa-inside.md"),
            "---\ntitle: Inside\n---\n\nbody\n",
        )
        .unwrap();
        std::fs::write(
            base.join("elsewhere/bbbb-outside.md"),
            "---\ntitle: Outside\n---\n\nbody\n",
        )
        .unwrap();

        let root = Utf8PathBuf::try_from(base.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();

        // No read yet, so the handle is unopened. Swap the directory
        // for a link out of the store.
        std::fs::remove_dir_all(base.join("store/.zettel")).unwrap();
        std::os::unix::fs::symlink(base.join("elsewhere"), base.join("store/.zettel")).unwrap();

        let ids: Vec<String> = repo
            .list_notes(&ListNotesFilter::default())
            .map(|v| v.into_iter().map(|n| n.id).collect())
            .unwrap_or_default();
        assert!(
            !ids.iter().any(|i| i == "bbbb-outside"),
            "a note from outside the store was listed: {ids:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A directory that cannot be read is a fault, not an empty store.
    ///
    /// Swallowing every open failure reported zero notes with a
    /// success status, so a person whose notes vanished got no signal.
    #[test]
    fn an_unreadable_note_directory_is_an_error_not_an_empty_store() {
        let base = fixture("unreadable");
        std::fs::create_dir_all(base.join("store/.zettel")).unwrap();
        std::fs::write(base.join("store/zettel.yml"), "zettel_dir: .zettel\n").unwrap();
        std::fs::write(
            base.join("store/.zettel/aaaa-real.md"),
            "---\ntitle: Real\n---\n\nbody\n",
        )
        .unwrap();

        let root = Utf8PathBuf::try_from(base.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();
        assert_eq!(
            repo.list_notes(&ListNotesFilter::default()).unwrap().len(),
            1
        );

        let dir = base.join("store/.zettel");
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
        std::fs::set_permissions(&dir, perms).unwrap();

        // A fresh Repo, so the handle is not already cached.
        let repo = Repo::open(&root).unwrap();
        let listed = repo.list_notes(&ListNotesFilter::default());
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&dir, perms).unwrap();

        assert!(
            listed.is_err(),
            "an unreadable note directory listed as an empty store"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The MCP server holds a Repo across threads.
    #[test]
    fn a_repo_stays_shareable_between_threads() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Repo>();
    }

    /// The id guard and the handle are two lines of defence. Removing
    /// either alone left the whole suite green, and removing both let
    /// `note delete ../../outside/secret` remove a file outside the
    /// store. Each is asserted here on its own.
    #[test]
    fn an_id_that_spells_a_path_is_refused_before_it_reaches_the_store() {
        let (base, repo) = store_with_one_note("idguard");
        // The empty id and a dot name are the guard's own work: both
        // are inside the store, so the handle cannot refuse them. With
        // the guard gone, `note delete ''` removes .zettel/.md and
        // `note delete .hidden` removes .zettel/.hidden.md.
        std::fs::write(base.join("store/.zettel/.hidden.md"), "dot").unwrap();
        std::fs::write(base.join("store/.zettel/.md"), "empty").unwrap();
        for bad in [
            "../../secret",
            "/etc/passwd",
            "..",
            ".",
            "a/b",
            "",
            ".hidden",
        ] {
            assert!(repo.resolve_id(bad).is_err(), "{bad} resolved to a note id");
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_delete_cannot_remove_a_file_outside_the_store() {
        let (base, repo) = store_with_one_note("delguard");
        std::fs::write(base.join("secret.md"), "SECRET").unwrap();

        for bad in ["../../secret", "../secret", "/etc/hosts"] {
            assert!(repo.delete_note(bad).is_err(), "{bad} was deleted");
        }
        assert_eq!(
            std::fs::read_to_string(base.join("secret.md")).unwrap(),
            "SECRET",
            "a file outside the store was removed"
        );

        // The name goes through the handle even when the id guard is
        // not what refuses it: a plain stem naming nothing is a miss,
        // and the real note still deletes.
        assert!(repo.delete_note("nosuchnote").is_err());
        assert!(repo.delete_note("aaaa-real").is_ok());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A link planted among the notes is not a note. It must not be
    /// listed, must not resolve, and must not be deleted through
    /// zettel, whatever it points at.
    #[test]
    fn a_planted_link_is_never_listed_resolved_or_deleted() {
        let (base, repo) = store_with_one_note("planted");
        std::fs::write(base.join("secret.md"), "SECRET").unwrap();
        std::os::unix::fs::symlink(
            base.join("secret.md"),
            base.join("store/.zettel/aaaa-planted.md"),
        )
        .unwrap();

        let ids: Vec<String> = repo
            .list_notes(&ListNotesFilter::default())
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["aaaa-real".to_string()], "a link was listed");
        assert!(repo.resolve_id("aaaa-planted").is_err(), "a link resolved");
        // By slug and by prefix too. These go through the scan rather
        // than the exact-match test, so they are what covers the
        // scan's own type filter.
        assert!(
            repo.resolve_id("planted").is_err(),
            "a link resolved by slug"
        );
        assert!(
            repo.resolve_id("aaaa").is_ok(),
            "the real note stopped resolving by prefix"
        );
        assert!(
            repo.delete_note("aaaa-planted").is_err(),
            "a link was deleted"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("secret.md")).unwrap(),
            "SECRET"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    fn fixture(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("zettel-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    fn store_with_one_note(tag: &str) -> (std::path::PathBuf, Repo) {
        let base = fixture(tag);
        std::fs::create_dir_all(base.join("store/.zettel")).unwrap();
        std::fs::write(base.join("store/zettel.yml"), "zettel_dir: .zettel\n").unwrap();
        std::fs::write(
            base.join("store/.zettel/aaaa-real.md"),
            "---\ntitle: Real\n---\n\nbody\n",
        )
        .unwrap();
        let root = Utf8PathBuf::try_from(base.join("store")).unwrap();
        let repo = Repo::open(&root).unwrap();
        (base, repo)
    }
}

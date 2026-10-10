//! Writing many notes in one run.
//!
//! A loop of [`Repo::create_note`] calls lists the note directory once
//! per note. Its cost grows with the square of the notes written.
//! A batch lists the directory once and keeps the ids in use in memory.
//! It claims each new id in constant time.
//!
//! [`KeyedBatch`] adds a key: one frontmatter field, such as `url`.
//! The key identifies the source record that a note came from.
//! [`KeyedBatch::upsert`] creates the note for a new record and updates
//! the note for a changed record. It writes nothing for an unchanged
//! record. A loader can run again over the same source and get the
//! same store.
//!
//! # What a caller must know
//!
//! 1. **One writer.** The batch reads the store once, when it opens.
//!    It assumes that it is the only writer until it is dropped. A
//!    write replaces the file at its path. A note that another process
//!    writes under the same id during the run is lost.
//! 2. **Key values match byte for byte.** Normalize them before the
//!    upsert. Two spellings of one URL are two records.
//! 3. **A deleted note comes back.** When a person deletes a loaded
//!    note, the next upsert of its record creates it again.
//!
//! # The record mapping
//!
//! A loaded note is a note that the batch wrote. Its frontmatter holds
//! a `record` mapping with these items:
//!
//! 1. A digest of the content: the body and the provenance spec.
//! 2. A digest of the metadata: the title, tags, links and fields.
//! 3. A digest of the body as written.
//! 4. The tags, links and field names that the record supplied.
//!
//! The batch updates only a note with this mapping. It never takes
//! over a note without it.
//!
//! # Review stamps
//!
//! An unchanged record writes nothing. A metadata change does not
//! touch the body or the provenance spec. A review stamp survives both.
//! A content change replaces the body and the spec. The stamps go with
//! the text that they vouched for.
//!
//! # Conflicts
//!
//! A person or a review can change the body of a loaded note. When the
//! record's content then changes, the batch keeps the body on disk. The
//! upsert reports [`ConflictReason::BodyChanged`] on each run until a
//! person resolves it. There are two ways:
//!
//! 1. **Take the source text.** Set the body to the new body of the
//!    record. The next upsert accepts it as current and keeps its
//!    review stamps.
//! 2. **Keep the hand text.** Remove the `record` mapping from the
//!    frontmatter. The note is then a person's note. Each later upsert
//!    of its key reports [`ConflictReason::NotLoaded`] and writes
//!    nothing.
//!
//! # Example
//!
//! ```no_run
//! use zettel::{NoteRecord, Repo, Upserted};
//!
//! # fn main() -> zettel::Result<()> {
//! let repo = Repo::open(camino::Utf8Path::new("notes"))?;
//! let mut batch = repo.keyed_batch("url")?;
//! let mut fields = yaml_serde::Mapping::new();
//! fields.insert("url".into(), "https://example.com/a".into());
//! fields.insert("outlet".into(), "Example".into());
//! let outcome = batch.upsert(NoteRecord {
//!     title: "An article".into(),
//!     body: "A summary of the article.".into(),
//!     provenance: Some("citation src=https://example.com/a".into()),
//!     tags: vec!["reading".into()],
//!     links: vec![],
//!     fields,
//! })?;
//! if let Upserted::Conflict { id, reason } = outcome {
//!     eprintln!("{id}: {reason:?}");
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, HashSet};

use mdstore::slug::{extract_prefix, generate_prefix, slugify};
use yaml_serde::{Mapping, Value};

use crate::error::{Error, Result};
use crate::note;
use crate::repo::{CreateNoteOptions, Repo};

/// The longest slug that a title gives, in bytes.
///
/// A file name has a limit, and a title does not. A keyed batch can add
/// a suffix of seven or more bytes to separate two records with one
/// title. The limit leaves room for that suffix, the prefix and the
/// extension.
pub const MAX_SLUG_BYTES: usize = 100;

/// The frontmatter key that holds a loaded note's record state.
pub const RECORD_KEY: &str = "record";

/// Frontmatter names a record cannot set.
///
/// The first six are the fields zettel models. `status` is removed by
/// `zettel migrate`. `record` holds the state this module writes.
pub const RESERVED_FIELDS: [&str; 8] = [
    "title",
    "tags",
    "links",
    "provenance",
    "created",
    "updated",
    "status",
    RECORD_KEY,
];

/// The slug of a title, cut to [`MAX_SLUG_BYTES`] at a word boundary.
///
/// A slug with no `-` inside the limit is cut at the limit.
pub(crate) fn bounded_slug(title: &str) -> String {
    let slug = slugify(title);
    if slug.len() <= MAX_SLUG_BYTES {
        return slug;
    }
    // slugify emits ASCII only, so every byte offset is a char boundary.
    let head = &slug[..MAX_SLUG_BYTES];
    // A word that ends exactly at the limit is whole.
    if slug.as_bytes()[MAX_SLUG_BYTES] == b'-' {
        return head.to_string();
    }
    // slugify trims a leading '-', so a match is never at 0.
    match head.rfind('-') {
        Some(i) => head[..i].to_string(),
        None => head.to_string(),
    }
}

/// The ids in use in one store, held for one run.
///
/// Crate-private: [`Repo::create_note`] uses it for one note, and
/// [`KeyedBatch`] for many.
pub(crate) struct Batch<'a> {
    repo: &'a Repo,
    prefixes: HashSet<String>,
    slugs: HashSet<String>,
}

impl<'a> Batch<'a> {
    /// List the store once and hold what is in use.
    pub(crate) fn open(repo: &'a Repo) -> Result<Self> {
        let stems = repo.note_stems()?;
        Ok(Self::from_stems(repo, &stems))
    }

    fn from_stems(repo: &'a Repo, stems: &[String]) -> Self {
        let mut prefixes = HashSet::with_capacity(stems.len());
        let mut slugs = HashSet::with_capacity(stems.len());
        for stem in stems {
            match extract_prefix(stem) {
                Some((prefix, slug)) => {
                    prefixes.insert(prefix.to_string());
                    slugs.insert(slug.to_string());
                }
                // A stem with no prefix holds its whole name as a slug,
                // so a new note cannot take that slug either.
                None => {
                    slugs.insert(stem.clone());
                }
            }
        }
        Batch {
            repo,
            prefixes,
            slugs,
        }
    }

    /// A random prefix no note uses.
    ///
    /// `generate_prefix` scans the slice it is given. An empty slice
    /// makes each draw constant time, and the set decides a hit.
    fn new_prefix(&mut self) -> String {
        self.new_prefix_from(|| generate_prefix(&[]))
    }

    /// Draw until a prefix is free. The draw is a parameter so a test
    /// can make it repeat.
    fn new_prefix_from(&mut self, mut draw: impl FnMut() -> String) -> String {
        loop {
            let prefix = draw();
            if self.prefixes.insert(prefix.clone()) {
                return prefix;
            }
        }
    }

    /// Claim a free slug under a new prefix, and return the id.
    fn claim(&mut self, slug: String) -> String {
        let prefix = self.new_prefix();
        let id = format!("{prefix}-{slug}");
        self.slugs.insert(slug);
        id
    }

    /// Create one note, under the rules of [`Repo::create_note`].
    pub(crate) fn create(&mut self, title: &str, opts: CreateNoteOptions) -> Result<String> {
        let slug = bounded_slug(title);
        if self.slugs.contains(&slug) {
            return Err(Error::NoteAlreadyExists(slug));
        }
        let id = self.claim(slug);
        self.repo.write_new_note(&id, title, opts)?;
        Ok(id)
    }
}

/// One source record, as a loader hands it to [`KeyedBatch::upsert`].
#[derive(Debug, Clone, Default)]
pub struct NoteRecord {
    pub title: String,
    /// The note body. Leading and trailing white space is not kept.
    pub body: String,
    /// The default provenance spec for the body. A review stamp in it
    /// is refused: only a review writes one.
    pub provenance: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    /// Frontmatter fields. Must hold the batch key as a string, and
    /// must not use a name in [`RESERVED_FIELDS`].
    pub fields: Mapping,
}

/// What one upsert did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upserted {
    /// No note held the key value. A new note holds it now.
    Created(String),
    /// The note changed to match the record.
    Updated(String),
    /// The note already matched the record. Nothing was written.
    Unchanged(String),
    /// The note does not match the record. The reason says what was
    /// written, if anything.
    Conflict { id: String, reason: ConflictReason },
}

/// Why an upsert left a note out of line with its record.
///
/// The [module documentation](self) says how a person resolves each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictReason {
    /// A note without a `record` mapping holds the key value. Nothing
    /// was written to it.
    NotLoaded,
    /// The record's content changed, and the body on disk is not the
    /// body last written. The body and the provenance spec stay as
    /// they are. A metadata change in the same record was applied.
    BodyChanged,
}

/// What a loaded note's `record` mapping holds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordState {
    content: String,
    meta: String,
    body: String,
    tags: Vec<String>,
    links: Vec<String>,
    /// The field names the record supplied, so a field the source
    /// drops can be removed without touching one a person added.
    fields: Vec<String>,
}

impl RecordState {
    fn from_yaml(value: &Value) -> Option<Self> {
        let map = value.as_mapping()?;
        let text = |k: &str| map.get(k).and_then(Value::as_str).map(str::to_string);
        let list = |k: &str| -> Option<Vec<String>> {
            map.get(k)?
                .as_sequence()?
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect()
        };
        Some(RecordState {
            content: text("content")?,
            meta: text("meta")?,
            body: text("body")?,
            tags: list("tags")?,
            links: list("links")?,
            fields: list("fields")?,
        })
    }

    fn to_yaml(&self) -> Value {
        let mut map = Mapping::new();
        map.insert("content".into(), self.content.clone().into());
        map.insert("meta".into(), self.meta.clone().into());
        map.insert("body".into(), self.body.clone().into());
        let seq = |v: &[String]| Value::Sequence(v.iter().map(|s| s.clone().into()).collect());
        map.insert("tags".into(), seq(&self.tags));
        map.insert("links".into(), seq(&self.links));
        map.insert("fields".into(), seq(&self.fields));
        Value::Mapping(map)
    }
}

/// One key value's place in the store.
enum Entry {
    /// One note holds the value. `state` is `None` for a note this
    /// module did not write.
    One {
        id: String,
        state: Option<RecordState>,
    },
    /// More than one note holds the value.
    Duplicate(Vec<String>),
}

/// A batch that finds notes by one frontmatter key and upserts records
/// against it. Open one with [`Repo::keyed_batch`].
///
/// The batch reads the store once, when it opens, and assumes it is
/// the only writer until it is dropped. See the [module
/// documentation](self) for what that means and what a loaded note
/// carries.
pub struct KeyedBatch<'a> {
    batch: Batch<'a>,
    key: String,
    entries: HashMap<String, Entry>,
}

impl Repo {
    /// Open a batch keyed on one frontmatter field.
    ///
    /// Lists the store once and reads every note once. A note that does
    /// not parse fails the open and is named: skipping it would let the
    /// next upsert of its record write a second note for one key.
    ///
    /// The batch assumes it is the only writer to the store until it
    /// is dropped.
    pub fn keyed_batch(&self, key: &str) -> Result<KeyedBatch<'_>> {
        check_field_name(key)?;
        let stems = self.note_stems()?;
        let mut entries: HashMap<String, Entry> = HashMap::new();
        for stem in &stems {
            let unreadable = |reason: String| Error::UnreadableNote {
                id: stem.clone(),
                reason,
            };
            let text = self
                .read_note_file(&self.note_path(stem))
                .map_err(|e| unreadable(e.to_string()))?;
            let (fm, _) = note::parse_note(&text).map_err(|e| unreadable(e.to_string()))?;
            let Some(value) = fm.extra.get(key).and_then(Value::as_str) else {
                continue;
            };
            let state =
                match fm.extra.get(RECORD_KEY) {
                    None => None,
                    Some(v) => Some(RecordState::from_yaml(v).ok_or_else(|| {
                        unreadable(format!("its '{RECORD_KEY}' field is malformed"))
                    })?),
                };
            match entries.get_mut(value) {
                None => {
                    entries.insert(
                        value.to_string(),
                        Entry::One {
                            id: stem.clone(),
                            state,
                        },
                    );
                }
                Some(Entry::One { id, .. }) => {
                    let ids = vec![id.clone(), stem.clone()];
                    entries.insert(value.to_string(), Entry::Duplicate(ids));
                }
                Some(Entry::Duplicate(ids)) => ids.push(stem.clone()),
            }
        }
        Ok(KeyedBatch {
            batch: Batch::from_stems(self, &stems),
            key: key.to_string(),
            entries,
        })
    }
}

/// Refuse a field name that is empty or that zettel owns.
fn check_field_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidFieldName("an empty name".to_string()));
    }
    if RESERVED_FIELDS.contains(&name) {
        return Err(Error::ReservedField(name.to_string()));
    }
    Ok(())
}

/// The spec with its review stamp removed, as text.
fn without_stamp(spec: Option<&str>) -> Option<String> {
    let mut marker = crate::provenance::parse_spec(spec?).ok()?;
    marker.attrs.retain(|(k, _)| {
        k != crate::provenance::ATTR_REVIEWED && k != crate::provenance::ATTR_REVIEWER
    });
    Some(marker.to_string())
}

/// `sha256:<hex>` over a JSON value.
fn digest(value: &serde_json::Value) -> String {
    mdstore::mcp::digest_of(value.to_string().as_bytes())
}

/// The hex part of a digest.
fn hex(digest: &str) -> &str {
    digest.strip_prefix("sha256:").unwrap_or(digest)
}

/// The names of a record's fields. The upsert checks first that each
/// is text.
fn field_names(record: &NoteRecord) -> Vec<String> {
    record
        .fields
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// Add `now`, remove what `before` held and `now` does not, and keep
/// everything else, in its order.
fn merge_supplied(current: &[String], before: &[String], now: &[String]) -> Vec<String> {
    let mut out: Vec<String> = current
        .iter()
        .filter(|t| now.contains(t) || !before.contains(t))
        .cloned()
        .collect();
    for t in now {
        if !out.contains(t) {
            out.push(t.clone());
        }
    }
    out
}

impl KeyedBatch<'_> {
    /// Create, update, or leave alone the note for one record.
    ///
    /// Checks the record before anything is written. Each failure is an
    /// error, and the batch carries on with the next record:
    ///
    /// 1. A field name that is not text, or is empty:
    ///    [`Error::InvalidFieldName`].
    /// 2. A field name that zettel owns: [`Error::ReservedField`].
    /// 3. No text value for the key: [`Error::MissingKey`].
    /// 4. A provenance spec or body marker that is not valid, or that
    ///    carries a review stamp: [`Error::InvalidProvenance`].
    /// 5. A key value that two notes hold: [`Error::DuplicateKey`].
    pub fn upsert(&mut self, record: NoteRecord) -> Result<Upserted> {
        for name in record.fields.keys() {
            let name = name
                .as_str()
                .ok_or_else(|| Error::InvalidFieldName(format!("{name:?}")))?;
            check_field_name(name)?;
        }
        let value = record
            .fields
            .get(self.key.as_str())
            .and_then(Value::as_str)
            .ok_or_else(|| Error::MissingKey(self.key.clone()))?
            .to_string();
        if let Some(spec) = &record.provenance {
            crate::provenance::parse_authored_spec(spec)?;
        }
        crate::provenance::refuse_authored_stamps(&record.body)?;
        crate::provenance::resolve_spans(record.provenance.as_deref(), &record.body)?;

        let body = record.body.trim();
        let content = digest(&serde_json::json!([record.provenance, body]));
        let meta = self.meta_digest(&record)?;

        match self.entries.get(&value) {
            None => self.create(&record, &value, content, meta),
            Some(Entry::Duplicate(ids)) => Err(Error::DuplicateKey {
                value,
                ids: ids.clone(),
            }),
            Some(Entry::One { id, state: None }) => Ok(Upserted::Conflict {
                id: id.clone(),
                reason: ConflictReason::NotLoaded,
            }),
            Some(Entry::One {
                id,
                state: Some(state),
            }) => {
                if state.content == content && state.meta == meta {
                    return Ok(Upserted::Unchanged(id.clone()));
                }
                let (id, state) = (id.clone(), state.clone());
                self.update(&record, &value, id, state, content, meta)
            }
        }
    }

    fn meta_digest(&self, record: &NoteRecord) -> Result<String> {
        let mut fields: Vec<(String, String)> = Vec::with_capacity(record.fields.len());
        for (k, v) in &record.fields {
            let name = k.as_str().unwrap_or_default().to_string();
            fields.push((name, yaml_serde::to_string(v)?));
        }
        fields.sort();
        Ok(digest(&serde_json::json!([
            record.title,
            record.tags,
            record.links,
            fields
        ])))
    }

    /// A free slug for a new record.
    ///
    /// The title's slug when it is free. Otherwise the slug and six hex
    /// digits of the key value's digest, widened by two while that is
    /// in use. A title with no ASCII letters or digits gets `k` and
    /// eight digits. The answer depends only on the title, the key
    /// value and the slugs in use.
    fn slug_for(&self, title: &str, value: &str) -> Result<String> {
        let key_digest = mdstore::mcp::digest_of(value.as_bytes());
        let digits = hex(&key_digest);
        let base = bounded_slug(title);
        let (lead, mut n) = if base.is_empty() {
            ("k".to_string(), 8)
        } else if !self.batch.slugs.contains(&base) {
            return Ok(base);
        } else {
            (format!("{base}-"), 6)
        };
        while n <= digits.len() {
            let slug = format!("{lead}{}", &digits[..n]);
            if !self.batch.slugs.contains(&slug) {
                return Ok(slug);
            }
            n += 2;
        }
        Err(Error::NoteAlreadyExists(format!("{lead}{digits}")))
    }

    fn create(
        &mut self,
        record: &NoteRecord,
        value: &str,
        content: String,
        meta: String,
    ) -> Result<Upserted> {
        let slug = self.slug_for(&record.title, value)?;
        let id = self.batch.claim(slug);
        let body = record.body.trim();
        let state = RecordState {
            content,
            meta,
            body: mdstore::mcp::digest_of(body.as_bytes()),
            tags: record.tags.clone(),
            links: record.links.clone(),
            fields: field_names(record),
        };
        let mut fm = note::new_frontmatter(&record.title);
        fm.provenance.clone_from(&record.provenance);
        fm.tags.clone_from(&record.tags);
        fm.links.clone_from(&record.links);
        for (k, v) in &record.fields {
            fm.extra.insert(k.clone(), v.clone());
        }
        fm.extra.insert(RECORD_KEY.into(), state.to_yaml());
        let repo = self.batch.repo;
        repo.write_note_file(&repo.note_path(&id), &note::serialize_note(&fm, body))?;
        self.entries.insert(
            value.to_string(),
            Entry::One {
                id: id.clone(),
                state: Some(state),
            },
        );
        Ok(Upserted::Created(id))
    }

    fn update(
        &mut self,
        record: &NoteRecord,
        value: &str,
        id: String,
        before: RecordState,
        content: String,
        meta: String,
    ) -> Result<Upserted> {
        let repo = self.batch.repo;
        let path = repo.note_path(&id);
        let (mut fm, mut body) = note::parse_note(&repo.read_note_file(&path)?)?;
        let mut state = before.clone();
        let mut conflict = false;

        if before.content != content {
            let source = record.body.trim();
            if mdstore::mcp::digest_of(body.as_bytes()) == before.body {
                // The body is the one last written, so nobody has
                // changed it. Replace it and the spec; their stamps
                // vouched for text that is no longer there.
                body = source.to_string();
                fm.provenance.clone_from(&record.provenance);
                state.content = content;
                state.body = mdstore::mcp::digest_of(body.as_bytes());
            } else if body == source {
                // A person already set the body to the source text.
                // Accept it as current. A stamp on the spec vouches for
                // this same text, so it stays when the spec otherwise
                // matches the record.
                let record_spec = without_stamp(record.provenance.as_deref());
                if without_stamp(fm.provenance.as_deref()) != record_spec {
                    fm.provenance.clone_from(&record.provenance);
                }
                state.content = content;
                state.body = mdstore::mcp::digest_of(body.as_bytes());
            } else {
                conflict = true;
            }
        }

        if before.meta != meta {
            fm.title.clone_from(&record.title);
            let now = field_names(record);
            for dropped in before.fields.iter().filter(|f| !now.contains(f)) {
                fm.extra.remove(dropped.as_str());
            }
            for (k, v) in &record.fields {
                fm.extra.insert(k.clone(), v.clone());
            }
            fm.tags = merge_supplied(&fm.tags, &before.tags, &record.tags);
            fm.links = merge_supplied(&fm.links, &before.links, &record.links);
            state.meta = meta;
            state.tags.clone_from(&record.tags);
            state.links.clone_from(&record.links);
            state.fields = now;
        }

        if state != before {
            crate::provenance::resolve_spans(fm.provenance.as_deref(), &body)?;
            fm.extra.insert(RECORD_KEY.into(), state.to_yaml());
            note::update_timestamp(&mut fm);
            repo.write_note_file(&path, &note::serialize_note(&fm, &body))?;
            self.entries.insert(
                value.to_string(),
                Entry::One {
                    id: id.clone(),
                    state: Some(state),
                },
            );
        }

        Ok(if conflict {
            Upserted::Conflict {
                id,
                reason: ConflictReason::BodyChanged,
            }
        } else {
            Upserted::Updated(id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::{EditNoteOptions, LISTINGS};
    use camino::Utf8PathBuf;

    struct Store {
        base: std::path::PathBuf,
        repo: Repo,
    }

    impl Drop for Store {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    impl Store {
        fn new(tag: &str) -> Self {
            let base =
                std::env::temp_dir().join(format!("zettel-batch-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            std::fs::create_dir_all(base.join(".zettel")).unwrap();
            std::fs::write(base.join("zettel.yml"), "zettel_dir: .zettel\n").unwrap();
            let root = Utf8PathBuf::try_from(base.clone()).unwrap();
            let repo = Repo::open(&root).unwrap();
            Store { base, repo }
        }

        fn path(&self, id: &str) -> std::path::PathBuf {
            self.base.join(format!(".zettel/{id}.md"))
        }

        fn text(&self, id: &str) -> String {
            std::fs::read_to_string(self.path(id)).unwrap()
        }

        fn mtime(&self, id: &str) -> std::time::SystemTime {
            std::fs::metadata(self.path(id))
                .unwrap()
                .modified()
                .unwrap()
        }

        fn plant(&self, name: &str, text: &str) {
            std::fs::write(self.base.join(format!(".zettel/{name}.md")), text).unwrap();
        }

        fn count(&self) -> usize {
            std::fs::read_dir(self.base.join(".zettel"))
                .unwrap()
                .filter(|e| {
                    e.as_ref()
                        .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "md"))
                })
                .count()
        }
    }

    fn record(url: &str, title: &str, body: &str) -> NoteRecord {
        let mut fields = Mapping::new();
        fields.insert("url".into(), url.into());
        fields.insert("outlet".into(), "Example".into());
        NoteRecord {
            title: title.into(),
            body: body.into(),
            provenance: Some("agent:summary".into()),
            tags: vec!["reading".into()],
            links: vec![],
            fields,
        }
    }

    fn id_of(outcome: &Upserted) -> &str {
        match outcome {
            Upserted::Created(id) | Upserted::Updated(id) | Upserted::Unchanged(id) => id,
            Upserted::Conflict { id, .. } => id,
        }
    }

    fn listings() -> usize {
        LISTINGS.with(std::cell::Cell::get)
    }

    #[test]
    fn a_batch_lists_the_store_once_however_many_notes_it_writes() {
        let s = Store::new("listonce");
        let before = listings();
        let mut batch = s.repo.keyed_batch("url").unwrap();
        for i in 0..2000 {
            let r = record(&format!("https://e.com/{i}"), &format!("Title {i}"), "b");
            assert!(matches!(batch.upsert(r).unwrap(), Upserted::Created(_)));
        }
        assert_eq!(listings() - before, 1, "the store was listed per note");
        assert_eq!(s.count(), 2000);
    }

    #[test]
    fn create_note_lists_the_store_once_per_note() {
        let s = Store::new("createonce");
        let before = listings();
        for i in 0..3 {
            s.repo
                .create_note(&format!("Note {i}"), CreateNoteOptions::default())
                .unwrap();
        }
        assert_eq!(listings() - before, 3);
    }

    #[test]
    #[ignore = "writes 40,000 files; run by hand"]
    fn forty_thousand_records_load_through_one_batch() {
        let s = Store::new("forty");
        let mut batch = s.repo.keyed_batch("url").unwrap();
        for i in 0..40_000 {
            let r = record(&format!("https://e.com/{i}"), &format!("Title {i}"), "b");
            batch.upsert(r).unwrap();
        }
        assert_eq!(s.count(), 40_000);
    }

    #[test]
    fn a_long_title_is_cut_at_a_word() {
        let s = Store::new("longtitle");
        let title = "word ".repeat(80);
        let id = s
            .repo
            .create_note(&title, CreateNoteOptions::default())
            .unwrap();
        let (_, slug) = extract_prefix(&id).unwrap();
        assert!(slug.len() <= MAX_SLUG_BYTES, "{} bytes", slug.len());
        assert!(slug.ends_with("word"), "{slug}");
        assert!(slugify(&title).starts_with(slug));
        // A slug with no word break is cut at the limit.
        assert_eq!(bounded_slug(&"x".repeat(300)).len(), MAX_SLUG_BYTES);
    }

    #[test]
    fn create_note_still_refuses_a_slug_in_use() {
        let s = Store::new("slugtaken");
        s.repo
            .create_note("Same", CreateNoteOptions::default())
            .unwrap();
        assert!(matches!(
            s.repo.create_note("Same", CreateNoteOptions::default()),
            Err(Error::NoteAlreadyExists(_))
        ));
        s.plant("bare", "---\ntitle: Bare\n---\n");
        assert!(matches!(
            s.repo.create_note("Bare", CreateNoteOptions::default()),
            Err(Error::NoteAlreadyExists(_))
        ));
    }

    #[test]
    fn upsert_creates_then_leaves_an_unchanged_record_alone() {
        let s = Store::new("rerun");
        let records: Vec<NoteRecord> = (0..3)
            .map(|i| record(&format!("https://e.com/{i}"), &format!("T {i}"), "body"))
            .collect();
        let mut ids = Vec::new();
        {
            let mut batch = s.repo.keyed_batch("url").unwrap();
            for r in &records {
                let out = batch.upsert(r.clone()).unwrap();
                assert!(matches!(out, Upserted::Created(_)), "{out:?}");
                ids.push(id_of(&out).to_string());
            }
        }
        for id in &ids {
            assert!(s.text(id).contains("record:"), "no record state on {id}");
        }
        let bytes: Vec<String> = ids.iter().map(|id| s.text(id)).collect();

        let mut batch = s.repo.keyed_batch("url").unwrap();
        for (r, id) in records.iter().zip(&ids) {
            assert_eq!(
                batch.upsert(r.clone()).unwrap(),
                Upserted::Unchanged(id.clone())
            );
        }
        let after: Vec<String> = ids.iter().map(|id| s.text(id)).collect();
        assert_eq!(bytes, after, "an unchanged record rewrote its note");
    }

    #[test]
    fn a_numeric_key_value_is_not_indexed() {
        let s = Store::new("numeric");
        s.plant("aaaa-num", "---\ntitle: Num\nurl: 5\n---\n\nbody\n");
        let mut batch = s.repo.keyed_batch("url").unwrap();
        let out = batch.upsert(record("5", "Five", "b")).unwrap();
        assert!(matches!(out, Upserted::Created(_)), "{out:?}");
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn a_key_value_two_notes_hold_fails_only_that_record() {
        let s = Store::new("dupkey");
        s.plant("aaaa-one", "---\ntitle: One\nurl: https://e.com/x\n---\n");
        s.plant("bbbb-two", "---\ntitle: Two\nurl: https://e.com/x\n---\n");
        let mut batch = s.repo.keyed_batch("url").unwrap();
        match batch.upsert(record("https://e.com/x", "X", "b")) {
            Err(Error::DuplicateKey { ids, .. }) => {
                assert_eq!(ids, vec!["aaaa-one".to_string(), "bbbb-two".to_string()]);
            }
            other => panic!("expected DuplicateKey, got {other:?}"),
        }
        assert!(matches!(
            batch.upsert(record("https://e.com/y", "Y", "b")).unwrap(),
            Upserted::Created(_)
        ));
    }

    #[test]
    fn a_record_without_its_key_or_with_a_reserved_field_writes_nothing() {
        let s = Store::new("badrecord");
        let mut batch = s.repo.keyed_batch("url").unwrap();

        let mut no_key = record("u", "T", "b");
        no_key.fields.remove("url");
        assert!(matches!(batch.upsert(no_key), Err(Error::MissingKey(k)) if k == "url"));

        for name in RESERVED_FIELDS {
            let mut r = record("u", "T", "b");
            r.fields.insert(name.into(), "x".into());
            assert!(
                matches!(batch.upsert(r), Err(Error::ReservedField(n)) if n == name),
                "{name} was accepted"
            );
        }
        assert_eq!(s.count(), 0);
        assert!(matches!(
            s.repo.keyed_batch("tags"),
            Err(Error::ReservedField(_))
        ));
    }

    #[test]
    fn an_unparseable_note_fails_the_open_and_is_named() {
        let s = Store::new("unparseable");
        s.plant("aaaa-broken", "---\ntitle: [\n---\n");
        match s.repo.keyed_batch("url") {
            Err(Error::UnreadableNote { id, .. }) => assert_eq!(id, "aaaa-broken"),
            Err(other) => panic!("expected UnreadableNote, got {other}"),
            Ok(_) => panic!("an unparseable note was skipped"),
        }
    }

    #[test]
    fn a_content_change_replaces_the_body_and_keeps_the_rest() {
        let s = Store::new("content");
        s.plant("cccc-other", "---\ntitle: Other\n---\n");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "old")).unwrap()).to_string()
        };
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    add_link: Some("cccc-other"),
                    ..Default::default()
                },
            )
            .unwrap();
        let created = s.repo.find_note(&id).unwrap().frontmatter.created;
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(
            b.upsert(record("u", "T", "new")).unwrap(),
            Upserted::Updated(id.clone())
        );
        let n = s.repo.find_note(&id).unwrap();
        assert_eq!(n.body, "new");
        assert_eq!(n.frontmatter.created, created);
        assert_eq!(n.frontmatter.links, vec!["cccc-other".to_string()]);
        assert_eq!(
            n.frontmatter.extra.get("outlet").and_then(Value::as_str),
            Some("Example")
        );
    }

    #[test]
    fn a_metadata_change_keeps_the_body_and_its_stamp() {
        let s = Store::new("metaonly");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "text")).unwrap()).to_string()
        };
        s.repo.approve_spans(&id, None, Some("dana")).unwrap();
        let mut r = record("u", "New title", "text");
        r.fields.insert("outlet".into(), "Other".into());
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), Upserted::Updated(id.clone()));
        let n = s.repo.find_note(&id).unwrap();
        assert_eq!(n.frontmatter.title, "New title");
        assert_eq!(
            n.frontmatter.extra.get("outlet").and_then(Value::as_str),
            Some("Other")
        );
        assert!(
            n.frontmatter.provenance.unwrap().contains("reviewed="),
            "a metadata change dropped the stamp"
        );
    }

    #[test]
    fn stamps_survive_an_unchanged_record() {
        let s = Store::new("stampkept");
        let r = record(
            "u",
            "T",
            "alpha\n\n<!-- prov agent:inference -->\nbeta\n<!-- /prov -->",
        );
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(r.clone()).unwrap()).to_string()
        };
        // Span 1 by number wraps the unmarked text in its own stamped
        // marker, which rewrites the body. Then the default.
        s.repo.approve_spans(&id, Some(&[1]), None).unwrap();
        s.repo.approve_spans(&id, None, None).unwrap();
        let stamped = s.text(&id);
        assert!(stamped.matches("reviewed=").count() >= 2, "{stamped}");

        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), Upserted::Unchanged(id.clone()));
        assert_eq!(s.text(&id), stamped);
    }

    #[test]
    fn a_content_change_on_an_untouched_body_drops_every_stamp() {
        let s = Store::new("stampdropped");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "text")).unwrap()).to_string()
        };
        s.repo.approve_spans(&id, None, None).unwrap();
        assert!(s.text(&id).contains("reviewed="));
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(
            b.upsert(record("u", "T", "revised")).unwrap(),
            Upserted::Updated(id.clone())
        );
        assert!(!s.text(&id).contains("reviewed="), "{}", s.text(&id));
    }

    #[test]
    fn a_body_changed_on_disk_is_never_overwritten() {
        let s = Store::new("handedit");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "loaded")).unwrap()).to_string()
        };
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    body: Some("written by hand"),
                    ..Default::default()
                },
            )
            .unwrap();

        let mut r = record("u", "T", "revised at the source");
        r.fields.insert("outlet".into(), "Other".into());
        let conflict = Upserted::Conflict {
            id: id.clone(),
            reason: ConflictReason::BodyChanged,
        };
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r.clone()).unwrap(), conflict);
        let n = s.repo.find_note(&id).unwrap();
        assert_eq!(n.body, "written by hand");
        assert_eq!(
            n.frontmatter.extra.get("outlet").and_then(Value::as_str),
            Some("Other"),
            "the metadata change was not applied"
        );
        // The conflict stays until a person resolves it.
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), conflict);
        assert_eq!(s.repo.find_note(&id).unwrap().body, "written by hand");
    }

    #[test]
    fn a_note_the_batch_did_not_write_is_never_taken_over() {
        let s = Store::new("notloaded");
        let text = "---\ntitle: Mine\nurl: u\n---\n\nmy own note\n";
        s.plant("aaaa-mine", text);
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(
            b.upsert(record("u", "T", "x")).unwrap(),
            Upserted::Conflict {
                id: "aaaa-mine".into(),
                reason: ConflictReason::NotLoaded
            }
        );
        assert_eq!(s.text("aaaa-mine"), text);
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn a_forged_stamp_in_a_record_is_refused() {
        let s = Store::new("forged");
        let mut b = s.repo.keyed_batch("url").unwrap();
        let mut r = record("u", "T", "x");
        r.provenance = Some("agent:summary reviewed=2026-01-01".into());
        assert!(matches!(b.upsert(r), Err(Error::InvalidProvenance(_))));
        let r = record(
            "u",
            "T",
            "<!-- prov agent:inference reviewed=2026-01-01 -->\nx\n<!-- /prov -->",
        );
        assert!(matches!(b.upsert(r), Err(Error::InvalidProvenance(_))));
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn two_records_with_one_title_both_land_in_either_order() {
        for (tag, urls) in [("sameab", ["a", "b"]), ("sameba", ["b", "a"])] {
            let s = Store::new(tag);
            let mut b = s.repo.keyed_batch("url").unwrap();
            let ids: Vec<String> = urls
                .iter()
                .map(|u| {
                    let out = b.upsert(record(u, "Same Title", "x")).unwrap();
                    assert!(matches!(out, Upserted::Created(_)), "{out:?}");
                    id_of(&out).to_string()
                })
                .collect();
            let (_, first) = extract_prefix(&ids[0]).unwrap();
            let (_, second) = extract_prefix(&ids[1]).unwrap();
            assert_eq!(first, "same-title");
            let digest = mdstore::mcp::digest_of(urls[1].as_bytes());
            assert_eq!(second, format!("same-title-{}", &hex(&digest)[..6]));
            assert_eq!(s.count(), 2);
        }
    }

    #[test]
    fn ids_are_stable_across_a_shuffled_rerun() {
        let s = Store::new("stable");
        let records: Vec<NoteRecord> = (0..10)
            .map(|i| record(&format!("u{i}"), &format!("T {}", i % 3), "x"))
            .collect();
        let mut first = HashMap::new();
        {
            let mut b = s.repo.keyed_batch("url").unwrap();
            for r in &records {
                let url = r.fields.get("url").unwrap().as_str().unwrap().to_string();
                first.insert(url, id_of(&b.upsert(r.clone()).unwrap()).to_string());
            }
        }
        let mut b = s.repo.keyed_batch("url").unwrap();
        for r in records.iter().rev() {
            let url = r.fields.get("url").unwrap().as_str().unwrap().to_string();
            assert_eq!(
                b.upsert(r.clone()).unwrap(),
                Upserted::Unchanged(first[&url].clone())
            );
        }
        assert_eq!(s.count(), 10);
    }

    #[test]
    fn a_title_with_no_ascii_letters_gets_a_key_slug() {
        let s = Store::new("emptyslug");
        let mut b = s.repo.keyed_batch("url").unwrap();
        let out = b.upsert(record("u", "日本語", "x")).unwrap();
        let (_, slug) = extract_prefix(id_of(&out)).expect("a valid prefix");
        let digest = mdstore::mcp::digest_of(b"u");
        assert_eq!(slug, format!("k{}", &hex(&digest)[..8]));
    }

    #[test]
    fn tags_a_person_added_stay_and_tags_the_source_dropped_go() {
        let s = Store::new("tags");
        let mut r = record("u", "T", "x");
        r.tags = vec!["a".into(), "b".into()];
        r.links = vec!["l1".into()];
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(r.clone()).unwrap()).to_string()
        };
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    add_tag: Some("mine"),
                    add_link: Some("l9"),
                    ..Default::default()
                },
            )
            .unwrap();
        r.tags = vec!["b".into(), "c".into()];
        r.links = vec!["l2".into()];
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), Upserted::Updated(id.clone()));
        let fm = s.repo.find_note(&id).unwrap().frontmatter;
        assert_eq!(fm.tags, vec!["b", "mine", "c"]);
        assert_eq!(fm.links, vec!["l9", "l2"]);
    }

    #[test]
    fn merge_supplied_keeps_order_and_foreign_entries() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            merge_supplied(&s(&["a", "x", "b"]), &s(&["a", "b"]), &s(&["b", "c"])),
            s(&["x", "b", "c"])
        );
    }

    #[test]
    fn one_key_twice_in_one_batch_writes_one_note() {
        let s = Store::new("twiceone");
        let mut b = s.repo.keyed_batch("url").unwrap();
        let first = b.upsert(record("u", "T", "x")).unwrap();
        let id = id_of(&first).to_string();
        assert_eq!(
            b.upsert(record("u", "T", "x")).unwrap(),
            Upserted::Unchanged(id.clone())
        );
        assert_eq!(s.count(), 1);
        // An update in the batch moves its index too, so a repeat of
        // the updated record is unchanged, not a conflict.
        assert_eq!(
            b.upsert(record("u", "T", "y")).unwrap(),
            Upserted::Updated(id.clone())
        );
        assert_eq!(
            b.upsert(record("u", "T", "y")).unwrap(),
            Upserted::Unchanged(id)
        );
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn a_prefix_in_use_is_drawn_again() {
        let s = Store::new("redraw");
        s.plant("aaaa-taken", "---\ntitle: Taken\n---\n");
        let mut batch = Batch::open(&s.repo).unwrap();
        let mut draws = ["aaaa", "aaaa", "bbbb", "bbbb", "cccc"].into_iter();
        let mut draw = || draws.next().unwrap().to_string();
        assert_eq!(batch.new_prefix_from(&mut draw), "bbbb");
        assert_eq!(batch.new_prefix_from(&mut draw), "cccc");
    }

    #[test]
    fn a_record_with_a_bad_body_marker_writes_nothing() {
        let s = Store::new("badmarker");
        let mut b = s.repo.keyed_batch("url").unwrap();
        let r = record("u", "T", "<!-- prov robot -->\nx\n<!-- /prov -->");
        assert!(matches!(b.upsert(r), Err(Error::InvalidProvenance(_))));
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn a_malformed_record_mapping_fails_the_open_and_is_named() {
        let s = Store::new("badrecordmap");
        s.plant(
            "aaaa-odd",
            "---\ntitle: Odd\nurl: u\nrecord:\n  content: 5\n---\n",
        );
        match s.repo.keyed_batch("url") {
            Err(Error::UnreadableNote { id, .. }) => assert_eq!(id, "aaaa-odd"),
            Err(other) => panic!("expected UnreadableNote, got {other}"),
            Ok(_) => panic!("a malformed record mapping was accepted"),
        }
    }

    #[test]
    fn a_suffixed_slug_in_use_is_widened() {
        let s = Store::new("widen");
        let digest = mdstore::mcp::digest_of(b"u");
        let six = &hex(&digest)[..6];
        s.plant("aaaa-same-title", "---\ntitle: Same Title\n---\n");
        s.plant(
            &format!("bbbb-same-title-{six}"),
            "---\ntitle: Same Title\n---\n",
        );
        let mut b = s.repo.keyed_batch("url").unwrap();
        let out = b.upsert(record("u", "Same Title", "x")).unwrap();
        let (_, slug) = extract_prefix(id_of(&out)).unwrap();
        assert_eq!(slug, format!("same-title-{}", &hex(&digest)[..8]));
    }

    #[test]
    fn a_standing_conflict_does_not_rewrite_the_note() {
        let s = Store::new("standing");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "loaded")).unwrap()).to_string()
        };
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    body: Some("by hand"),
                    ..Default::default()
                },
            )
            .unwrap();
        let mut b = s.repo.keyed_batch("url").unwrap();
        let first = b.upsert(record("u", "T", "revised")).unwrap();
        assert!(matches!(first, Upserted::Conflict { .. }), "{first:?}");
        let text = s.text(&id);
        let mtime = s.mtime(&id);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut b = s.repo.keyed_batch("url").unwrap();
        let second = b.upsert(record("u", "T", "revised")).unwrap();
        assert!(matches!(second, Upserted::Conflict { .. }), "{second:?}");
        assert_eq!(s.mtime(&id), mtime, "a standing conflict rewrote the file");
        assert_eq!(s.text(&id), text, "a standing conflict rewrote the note");
    }

    /// Load one record in its own batch and return the note's id.
    fn load(s: &Store, r: &NoteRecord) -> String {
        let mut b = s.repo.keyed_batch("url").unwrap();
        let out = b.upsert(r.clone()).unwrap();
        assert!(matches!(out, Upserted::Created(_)), "{out:?}");
        id_of(&out).to_string()
    }

    #[test]
    fn a_title_change_alone_is_an_update() {
        let s = Store::new("titleonly");
        let r = record("u", "Old title", "text");
        let id = load(&s, &r);
        let mut changed = r.clone();
        changed.title = "New title".into();
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(changed).unwrap(), Upserted::Updated(id.clone()));
        let n = s.repo.find_note(&id).unwrap();
        assert_eq!(n.frontmatter.title, "New title");
        assert_eq!(n.body, "text");
    }

    #[test]
    fn a_tag_change_alone_is_an_update() {
        let s = Store::new("tagonly");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut changed = r.clone();
        changed.tags = vec!["reading".into(), "later".into()];
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(changed).unwrap(), Upserted::Updated(id.clone()));
        let fm = s.repo.find_note(&id).unwrap().frontmatter;
        assert_eq!(fm.tags, vec!["reading", "later"]);
    }

    #[test]
    fn a_link_change_alone_is_an_update() {
        let s = Store::new("linkonly");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut changed = r.clone();
        changed.links = vec!["aaaa-project".into()];
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(changed).unwrap(), Upserted::Updated(id.clone()));
        let fm = s.repo.find_note(&id).unwrap().frontmatter;
        assert_eq!(fm.links, vec!["aaaa-project"]);
    }

    #[test]
    fn a_provenance_change_alone_is_an_update() {
        let s = Store::new("provonly");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut changed = r.clone();
        changed.provenance = Some("citation src=https://example.com/acme".into());
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(changed).unwrap(), Upserted::Updated(id.clone()));
        let n = s.repo.find_note(&id).unwrap();
        assert_eq!(
            n.frontmatter.provenance.as_deref(),
            Some("citation src=https://example.com/acme")
        );
        assert_eq!(n.body, "text");
    }

    #[test]
    fn an_updated_record_reruns_unchanged_in_a_new_batch() {
        let s = Store::new("rerunafter");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut changed = r.clone();
        changed.title = "New title".into();
        changed.tags = vec!["later".into()];
        changed.links = vec!["aaaa-project".into()];
        changed.body = "revised".into();
        changed.provenance = Some("citation src=https://example.com/acme".into());
        {
            let mut b = s.repo.keyed_batch("url").unwrap();
            assert_eq!(
                b.upsert(changed.clone()).unwrap(),
                Upserted::Updated(id.clone())
            );
            assert_eq!(
                b.upsert(changed.clone()).unwrap(),
                Upserted::Unchanged(id.clone())
            );
        }
        let text = s.text(&id);
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(changed).unwrap(), Upserted::Unchanged(id.clone()));
        assert_eq!(s.text(&id), text, "an unchanged record rewrote its note");
    }

    #[test]
    fn a_body_set_to_the_source_text_clears_the_conflict() {
        let s = Store::new("converge");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(record("u", "T", "loaded")).unwrap()).to_string()
        };
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    body: Some("by hand"),
                    ..Default::default()
                },
            )
            .unwrap();
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert!(matches!(
            b.upsert(record("u", "T", "revised")).unwrap(),
            Upserted::Conflict { .. }
        ));
        // A person takes the source text and approves it.
        s.repo
            .edit_note(
                &id,
                EditNoteOptions {
                    body: Some("revised"),
                    ..Default::default()
                },
            )
            .unwrap();
        s.repo.approve_spans(&id, None, None).unwrap();
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(
            b.upsert(record("u", "T", "revised")).unwrap(),
            Upserted::Updated(id.clone())
        );
        assert!(
            s.text(&id).contains("reviewed="),
            "the stamp on the accepted text was dropped"
        );
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(
            b.upsert(record("u", "T", "revised")).unwrap(),
            Upserted::Unchanged(id)
        );
    }

    #[test]
    fn a_field_the_source_drops_goes_and_a_field_a_person_added_stays() {
        let s = Store::new("fields");
        let mut r = record("u", "T", "x");
        r.fields.insert("blurb".into(), "old".into());
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(r.clone()).unwrap()).to_string()
        };
        let text = s.text(&id).replace("blurb: old", "blurb: old\nmine: kept");
        std::fs::write(s.path(&id), text).unwrap();
        r.fields.remove("blurb");
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), Upserted::Updated(id.clone()));
        let extra = s.repo.find_note(&id).unwrap().frontmatter.extra;
        assert!(!extra.contains_key("blurb"), "a dropped field stayed");
        assert_eq!(extra.get("mine").and_then(Value::as_str), Some("kept"));
    }

    #[test]
    fn a_field_name_that_is_not_text_or_is_empty_is_refused() {
        let s = Store::new("fieldname");
        let mut b = s.repo.keyed_batch("url").unwrap();
        let mut r = record("u", "T", "x");
        r.fields.insert(Value::Number(1.into()), "x".into());
        assert!(matches!(b.upsert(r), Err(Error::InvalidFieldName(_))));
        assert_eq!(s.count(), 0);
        assert!(matches!(
            s.repo.keyed_batch(""),
            Err(Error::InvalidFieldName(_))
        ));
    }

    #[test]
    fn a_body_with_surrounding_white_space_reruns_clean() {
        let s = Store::new("whitespace");
        let r = record("u", "T", "\n text \n\n");
        let id = {
            let mut b = s.repo.keyed_batch("url").unwrap();
            id_of(&b.upsert(r.clone()).unwrap()).to_string()
        };
        let mut b = s.repo.keyed_batch("url").unwrap();
        assert_eq!(b.upsert(r).unwrap(), Upserted::Unchanged(id.clone()));
        assert_eq!(
            b.upsert(record("u", "T", "\n other \n")).unwrap(),
            Upserted::Updated(id)
        );
    }

    fn set_body(s: &Store, id: &str, body: &str) {
        s.repo
            .edit_note(
                id,
                EditNoteOptions {
                    body: Some(body),
                    ..Default::default()
                },
            )
            .unwrap();
    }

    fn upsert_once(s: &Store, r: NoteRecord) -> Upserted {
        s.repo.keyed_batch("url").unwrap().upsert(r).unwrap()
    }

    #[test]
    fn a_record_revised_twice_updates_both_times() {
        let s = Store::new("revisedtwice");
        let id = load(&s, &record("u", "T", "x"));
        for body in ["y", "z"] {
            assert_eq!(
                upsert_once(&s, record("u", "T", body)),
                Upserted::Updated(id.clone()),
                "revising to {body}"
            );
            assert_eq!(s.repo.find_note(&id).unwrap().body, body);
        }
    }

    #[test]
    fn a_body_accepted_from_the_source_takes_the_next_revision() {
        let s = Store::new("acceptedthen");
        let id = load(&s, &record("u", "T", "loaded"));
        set_body(&s, &id, "revised");
        assert_eq!(
            upsert_once(&s, record("u", "T", "revised")),
            Upserted::Updated(id.clone())
        );
        assert_eq!(
            upsert_once(&s, record("u", "T", "revised")),
            Upserted::Unchanged(id.clone())
        );
        assert_eq!(
            upsert_once(&s, record("u", "T", "final")),
            Upserted::Updated(id.clone())
        );
        assert_eq!(s.repo.find_note(&id).unwrap().body, "final");
    }

    #[test]
    fn what_the_source_added_in_one_run_it_can_drop_in_the_next() {
        let s = Store::new("addthendrop");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut added = r.clone();
        added.tags.push("later".into());
        added.links.push("aaaa-project".into());
        added.fields.insert("blurb".into(), "acme".into());
        assert_eq!(upsert_once(&s, added), Upserted::Updated(id.clone()));
        let fm = s.repo.find_note(&id).unwrap().frontmatter;
        assert_eq!(fm.tags, vec!["reading", "later"]);
        assert_eq!(fm.links, vec!["aaaa-project"]);
        assert!(fm.extra.contains_key("blurb"));

        assert_eq!(upsert_once(&s, r), Upserted::Updated(id.clone()));
        let fm = s.repo.find_note(&id).unwrap().frontmatter;
        assert_eq!(fm.tags, vec!["reading"], "a dropped tag stayed");
        assert!(fm.links.is_empty(), "a dropped link stayed");
        assert!(!fm.extra.contains_key("blurb"), "a dropped field stayed");
    }

    #[test]
    fn a_body_taken_from_the_source_gets_the_new_spec() {
        let s = Store::new("takenspec");
        let id = load(&s, &record("u", "T", "loaded"));
        set_body(&s, &id, "revised");
        let mut r = record("u", "T", "revised");
        r.provenance = Some("citation src=https://example.com/acme".into());
        assert_eq!(upsert_once(&s, r), Upserted::Updated(id.clone()));
        assert_eq!(
            s.repo
                .find_note(&id)
                .unwrap()
                .frontmatter
                .provenance
                .as_deref(),
            Some("citation src=https://example.com/acme")
        );
    }

    #[test]
    fn field_order_does_not_change_a_record() {
        let s = Store::new("fieldorder");
        let r = record("u", "T", "text");
        let id = load(&s, &r);
        let mut reordered = r.clone();
        reordered.fields = Mapping::new();
        reordered.fields.insert("outlet".into(), "Example".into());
        reordered.fields.insert("url".into(), "u".into());
        let text = s.text(&id);
        let mtime = s.mtime(&id);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(upsert_once(&s, reordered), Upserted::Unchanged(id.clone()));
        assert_eq!(s.mtime(&id), mtime, "a reordered record rewrote the file");
        assert_eq!(s.text(&id), text);
    }

    #[test]
    fn an_update_moves_the_updated_timestamp() {
        let s = Store::new("timestamp");
        let id = load(&s, &record("u", "T", "text"));
        let old = "2020-01-01T00:00:00Z";
        let text = s.text(&id);
        let line = text
            .lines()
            .find(|l| l.starts_with("updated:"))
            .unwrap()
            .to_string();
        std::fs::write(s.path(&id), text.replace(&line, &format!("updated: {old}"))).unwrap();
        assert_eq!(
            s.repo
                .find_note(&id)
                .unwrap()
                .frontmatter
                .updated
                .as_deref(),
            Some(old)
        );
        assert_eq!(
            upsert_once(&s, record("u", "New title", "text")),
            Upserted::Updated(id.clone())
        );
        let updated = s.repo.find_note(&id).unwrap().frontmatter.updated;
        assert_ne!(updated.as_deref(), Some(old), "the timestamp did not move");
    }

    #[test]
    fn a_word_that_ends_at_the_limit_is_kept_whole() {
        let title = format!("x {} b", "a".repeat(98));
        assert_eq!(bounded_slug(&title), format!("x-{}", "a".repeat(98)));
    }
}

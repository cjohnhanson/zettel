---
title: batch create and keyed upsert for large loads
status: in_progress
priority: '2'
assignee: null
due_date: null
labels:
- feature
depends_on: []
created: 2026-10-10T14:26:45Z
updated: 2026-10-10T14:46:22Z
---

## Problem

A loader that writes tens of thousands of notes into one store cannot use the library today. Three gaps block it.

1. `Repo::create_note` lists the note directory twice for each note: once for `slug_exists` and once for `collect_existing_prefixes`. The prefix list also removes duplicates with a linear search. The cost of one create grows with the store, so N creates cost more than N squared. At 40,000 notes this is not usable.
2. No call finds a note by a value in its frontmatter, such as `url`. A loader that runs a second time either fails with `NoteAlreadyExists` or writes a duplicate. It cannot update a note in place.
3. The slug comes from the title only. Two records with the same title, for example two articles from different outlets, collide with `NoteAlreadyExists`.

## Scope

1. A batch type that holds one in-memory index of the store, so each create costs constant time plus the write.
2. A keyed batch that finds a note by one frontmatter key and upserts a record against it.
3. A slug rule that separates two records with the same title, and gives the same answer for the same input.
4. Review stamps stay on a note whose record did not change. They go when the record changes.

The CLI and the MCP server do not change. `note create` keeps its behaviour, with two differences: it lists the directory once instead of twice, and it cuts a slug longer than 100 bytes.

## Out of scope

1. A `note import` command. The loader is a library caller.
2. A `--where` namespace for arbitrary frontmatter keys.
3. Changes to the fsync on each write. That belongs to mdstore.

## Acceptance

1. A test creates 2,000 notes through a batch and asserts that the store was listed once, not once per note. An ignored test creates 40,000 notes.
2. A second upsert of the same records writes no file and reports every record as unchanged.
3. Two records with one title both land, under different ids, in any order.
4. A review stamp survives an unchanged upsert and a metadata-only change, and is removed by a content change.
5. A body changed by hand is never overwritten, and a note the loader did not write is never taken over.

The design is in the scratch notes.

## Scratch Notes

# Design: batch create and keyed upsert (revision 2)

## The caller

A Singer target reads records from a tap and writes one note for each record. The Singer `SCHEMA` message names `key_properties`: the fields that identify a record. A target upserts on those fields. This design gives that target one key field, held in note frontmatter, and an upsert against it.

Typical record: `url`, `title`, `author`, `outlet`, `date`, `blurb`, and sometimes full text. The loader maps `url` to the key, `title` to the title, the text or blurb to the body, and the rest to frontmatter fields.

## Public API

```rust
impl Repo {
    /// One directory listing and one read of every note.
    pub fn keyed_batch(&self, key: &str) -> Result<KeyedBatch<'_>>;
}

impl KeyedBatch<'_> {
    pub fn upsert(&mut self, record: NoteRecord) -> Result<Upserted>;
}

pub struct NoteRecord {
    pub title: String,
    pub body: String,
    /// The default provenance spec. A review stamp in it is refused.
    pub provenance: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    /// Frontmatter fields. Must hold the batch key as a string.
    pub fields: yaml_serde::Mapping,
}

pub enum Upserted {
    Created(String),
    Updated(String),
    Unchanged(String),
    Conflict { id: String, reason: ConflictReason },
}

pub enum ConflictReason {
    /// A note the loader did not write holds the key value.
    NotLoaded,
    /// The body on disk is not the body the loader last wrote.
    BodyChanged,
}
```

Crate-private: the unkeyed `Batch` that `create_note` uses. A lookup without an upsert has no caller, so `KeyedBatch` has no `find`; the tests reach the index through `upsert`.

New errors:

1. `MissingKey(key)`: the record has no string value for the batch key.
2. `DuplicateKey { value, ids }`: two notes in the store hold this value. The upsert for that value fails and names the ids. Other values proceed.
3. `ReservedField(name)`: a record field, or the batch key, uses a name zettel owns: `title`, `tags`, `links`, `provenance`, `created`, `updated`, `status` (which `migrate` removes), or `record`.
4. `InvalidFieldName(name)`: a record field name that is not text, or an empty batch key.
5. `keyed_batch` fails on a note that does not parse, or whose `record` mapping is malformed, and names it. A skipped note would be a second note for the same key on the next create.

## The index

Built once per batch:

1. Prefixes in use: `HashSet<String>`.
2. Slugs in use: `HashSet<String>`. A stem with no prefix counts as a slug in use, as it does in `slug_exists` now.
3. For a keyed batch, key value to entry. An entry holds the id, and, for a note the loader wrote, the stored record state (below). A value that two notes hold is marked as a duplicate.

A new prefix comes from `mdstore::slug::generate_prefix(&[])`, which then scans nothing, checked against the hash set and drawn again on a hit. mdstore does not change.

Each create checks and inserts in constant time, then writes one file. N creates cost N writes plus one listing.

### One writer

The index is a snapshot. The batch assumes it is the only writer to the store for the whole run. mdstore's write renames a staged file over the destination, so a note another process writes under the same id during the run is replaced. The doc comments on `Repo::keyed_batch` and `KeyedBatch` say so.

## Stored record state

A note the loader writes carries one frontmatter key, `record`, a mapping:

```yaml
record:
  content: sha256:...   # provenance spec and body, as the record gave them
  meta: sha256:...      # title, tags, links and fields, as the record gave them
  body: sha256:...      # the body as written to disk
  tags: [...]           # the tags the record supplied
  links: [...]          # the links the record supplied
  fields: [...]         # the field names the record supplied
```

Only a note with this key is the loader's. A note without it is never taken over.

Each digest is SHA-256 over a JSON array in a fixed order. Fields are sorted by name, and each value is serialized as YAML. The hash is `mdstore::mcp::digest_of`, so the change adds no dependency.

## Upsert

1. Check the record: the key value is a string, no field is reserved, the provenance spec and the body carry no review stamp, and the body markers are valid.
2. No note holds the value: create the note with its `record` key. `Created`.
3. Two notes hold the value: `DuplicateKey`.
4. A note without `record` holds the value: `Conflict { reason: NotLoaded }`. Nothing is written.
5. Both digests equal the stored ones: `Unchanged`. Nothing is read or written.
6. Otherwise read the note, then:
   1. Content digest changed and the body on disk matches the stored body digest: set the body and the provenance spec from the record. Store the new content and body digests.
   2. Content digest changed and the body on disk already equals the record's new body: a person took the source text. Accept it, store the new digests, and keep the spec on disk when it differs from the record's only by a review stamp.
   3. Content digest changed and the body on disk matches neither: a person or a review changed the body. Leave the body, the provenance spec, and the stored content digest as they are. The result is `Conflict { reason: BodyChanged }`, and the next run reports it again without a write.
   4. Meta digest changed: set the title and each record field, and remove each field the record supplied last time and does not supply now. Remove the tags and links the record supplied last time and does not supply now. Add the ones it supplies now. A field, tag or link a person added stays. Store the new meta digest and the supplied lists.

A person resolves `BodyChanged` in one of two ways: set the body to the source text (step 2), or remove the `record` mapping to keep the hand text (the note is then `NotLoaded`).
   4. Keep the id and `created`. Set `updated`. Write once.
   5. `Updated`, or `Conflict { BodyChanged }` from step 2.

### Review stamps

A stamp can sit on the frontmatter default spec, or on a body marker that `approve_spans` wrote for a named span.

1. Unchanged record: no write. Both kinds stay.
2. Metadata-only change: the body and the provenance spec are not touched. Both kinds stay.
3. Content change on an untouched body: the body and the spec come from the record, which carries no stamp. Every stamp goes, because the approved text is no longer there.
4. Content change on a body that an approval of a named span rewrote: the body differs from the stored body digest, so step 6.2 applies. The stamps and the approved text stay, and the result is a conflict a person resolves.

## Slugs

The rule for a new note in a keyed batch:

1. Slugify the title, as now, cut to 100 bytes at the last `-` before that length, or at 100 bytes when there is no `-`.
2. Empty slug: `k` and the first eight hex digits of the SHA-256 of the key value.
3. Slug not in use: use it.
4. Slug in use: append `-` and the first six hex digits of the SHA-256 of the key value. Widen by two digits while that slug is in use.

The result depends only on the title, the key value, and the slugs in use. A re-run never derives an id for a record already in the store; it finds the note by key. When two new records share a title, the first written gets the bare slug, and each keeps its id afterwards.

The four-character prefix stays random. A prefix derived from the key collides about 476 times in 40,000 notes over 36^4 values, and the probe result depends on order.

`create_note` keeps its rule: a slug in use is `NoteAlreadyExists`. It now cuts long slugs the same way.

## What a caller must know

1. Key values match byte for byte. Normalize them first; a URL loader strips fragments and tracking parameters itself.
2. A loaded note that a person deletes comes back on the next run of the same record.
3. One writer for the run.

## CLI and server

No change. `note create` uses the crate-private batch and keeps its output and errors.

## Options considered

1. A cached index inside `Repo`: stale with no signal under the MCP server and other processes. Rejected.
2. `create_note` faster and nothing else: still quadratic over N creates. Kept as part of the change, not enough alone.
3. Two notes with one slug: `resolve_id` stops finding either by slug. Rejected.
4. Number suffixes: depend on order and on deletes. Rejected.
5. Key suffix on every keyed note: longer ids for no gain over the key lookup. Rejected.
6. Suffix from another field such as `outlet`: specific to one loader, and can collide. Rejected.
7. Compare the record with the note on disk instead of storing digests: an approval rewrites the body without changing its content. Rejected.
8. One digest for the whole record: a changed tag would rewrite the body and drop every stamp. Rejected.
9. Replace tags and links on update: removes what a person added. Rejected.
10. Take over a note without `record` that holds the key: overwrites a note the loader never wrote. Rejected.

## Tests

1. One listing: a test-only counter on the directory listing. A batch creates 2,000 notes; the counter is 1. Through `create_note` the counter grows with each create.
2. 40,000 creates through a batch, `#[ignore]`, no timing assertion.
3. `create_note`: the existing tests pass; a long title is cut to 100 bytes at a word.
4. A numeric key value is not indexed: an upsert of the same text creates a note.
5. Duplicate key: two notes hold one url. Upsert of that url is `DuplicateKey`; another url succeeds.
6. Missing key and reserved field: errors, nothing written.
7. Unparseable note: `keyed_batch` fails and names it.
8. Create: three records, three `Created`, each with `record`.
9. Re-run: three `Unchanged`, the files byte for byte the same.
10. Content change: `Updated`, same id, new body, `created` and other keys kept.
11. Metadata-only change: `Updated`, the body and the default spec stamp kept.
12. Stamp kept when unchanged, by default and by named span.
13. Stamp dropped when content changes on an untouched body.
14. Body edited by hand, then content change: `Conflict { BodyChanged }`, body kept, a metadata change in the same record applied.
15. Note without `record` holding the key: `Conflict { NotLoaded }`, file unchanged.
16. Forged stamp in the record: `InvalidProvenance`, nothing written.
17. Same title twice: two notes, one bare slug, one suffixed; reverse order on a fresh store also lands both.
18. Stable ids across a shuffled re-run.
19. Empty slug: `k` and eight hex digits, a valid prefix.
20. Tags and links: a person-added tag stays; a tag the source dropped goes; a new source tag is added.

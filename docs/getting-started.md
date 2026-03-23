<!-- metadata
title: "Getting Started with Zettel"
description: "Initialize a knowledge base, create notes, link them, and explore the graph"
type: tutorial
-->

# Getting Started with Zettel

## Initialize

From your project root:

```bash
zettel init
```

This creates `zettel.yml` (config) and a `.zettel/` directory for notes.

## Create a note

```bash
zettel note create "Connection pooling causes stale reads under load" \
  -t debugging,postgres
```

Zettel prints the note ID (e.g., `a3f2-connection-pooling-causes-stale-reads`).
The note is created as a draft with the given title and tags. The 4-character
prefix (`a3f2`) is enough to identify the note in most commands.

To add a body inline:

```bash
zettel note create "Why we chose YAML over TOML" \
  -t architecture \
  -b "YAML supports anchors and aliases for config reuse. TOML can't."
```

## View and search

```bash
# List all notes
zettel note list

# Filter by tag
zettel note list --tag postgres

# Filter by status
zettel note list --status draft

# Show a specific note
zettel note show a3f2-connection-pooling-causes-stale-reads

# Full-text search
zettel search "connection pool"

# Dump all note content (useful for piping to other tools)
zettel read
zettel read --tag debugging
```

## Link notes

Notes connect to each other via the `links` frontmatter field. Add a link
when creating:

```bash
zettel note create "Workaround: force new connection per transaction" \
  -t postgres \
  -l a3f2-connection-pooling-causes-stale-reads
```

Or add links later:

```bash
zettel note edit b7c1-workaround-force-new-connection --add-link a3f2-connection-pooling-causes-stale-reads
```

## Explore the graph

```bash
# What links to this note?
zettel backlinks a3f2-connection-pooling-causes-stale-reads

# Show a note and its neighborhood (linked notes within 2 hops)
zettel context a3f2-connection-pooling-causes-stale-reads

# Deeper traversal
zettel context a3f2-connection-pooling-causes-stale-reads -d 4

# Find unlinked notes
zettel orphans

# Knowledge base health
zettel stats
```

## Edit and maintain

```bash
# Change title
zettel note edit a3f2-connection-pooling-causes-stale-reads --title "Connection pooling and stale reads"

# Add a tag
zettel note edit a3f2-connection-pooling-causes-stale-reads --add-tag production-incident

# Append to the body
zettel note edit a3f2-connection-pooling-causes-stale-reads --append "Confirmed: setting max_age=300 resolves this."

# Promote to permanent (human decision)
zettel note edit a3f2-connection-pooling-causes-stale-reads --status permanent

# Delete a note
zettel note delete a3f2-connection-pooling-causes-stale-reads
```

## The draft-to-permanent workflow

1. Agents and humans create notes as **drafts** during work
2. Periodically, review drafts: `zettel note list --status draft`
3. For each draft, decide: reformulate and promote, merge into another
   note, or delete
4. Promote with `zettel note edit <id> --status permanent`

The knowledge base stays useful when permanent notes are curated —
atomic, well-linked, and written in the author's voice.

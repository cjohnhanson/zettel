<!-- metadata
title: "Getting Started with Zettel"
description: "Initialize a knowledge base, create notes, link them, and explore the graph"
type: tutorial
-->

# Getting Started with Zettel

## Initialize

Run this command from the project root:

```bash
zettel init
```

The command makes the config file `zettel.yml` and the note directory
`.zettel/`.

## Create a note

```bash
zettel note create "Connection pooling causes stale reads under load" \
  -t debugging,postgres
```

Zettel prints the note ID, for example
`a3f2-connection-pooling-causes-stale-reads`. Zettel creates the note as a
draft with the given title and tags. Most commands accept the 4-character
prefix (`a3f2`) instead of the full ID.

Add a body on the command line:

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

# Show one note
zettel note show a3f2-connection-pooling-causes-stale-reads

# Search the full text
zettel search "connection pool"

# Show all note content; pipe it to other tools
zettel read
zettel read --tag debugging
```

## Link notes

Notes connect to each other through the `links` frontmatter field. Add a link
when you create the note:

```bash
zettel note create "Workaround: force new connection per transaction" \
  -t postgres \
  -l a3f2-connection-pooling-causes-stale-reads
```

Add a link to a note that already exists:

```bash
zettel note edit b7c1-workaround-force-new-connection --add-link a3f2-connection-pooling-causes-stale-reads
```

## Explore the graph

```bash
# Show the notes that link to this note
zettel backlinks a3f2-connection-pooling-causes-stale-reads

# Show a note and the linked notes within 2 hops
zettel context a3f2-connection-pooling-causes-stale-reads

# Traverse more hops
zettel context a3f2-connection-pooling-causes-stale-reads -d 4

# Show the notes with no links
zettel orphans

# Show the knowledge base statistics
zettel stats
```

## Edit and maintain

```bash
# Change the title
zettel note edit a3f2-connection-pooling-causes-stale-reads --title "Connection pooling and stale reads"

# Add a tag
zettel note edit a3f2-connection-pooling-causes-stale-reads --add-tag production-incident

# Append to the body
zettel note edit a3f2-connection-pooling-causes-stale-reads --append "Confirmed: setting max_age=300 resolves this."

# Promote the note to permanent; a human makes this decision
zettel note edit a3f2-connection-pooling-causes-stale-reads --status permanent

# Delete a note
zettel note delete a3f2-connection-pooling-causes-stale-reads
```

## The draft-to-permanent workflow

1. Agents and humans create notes as **drafts** during work.
2. Review the drafts regularly with `zettel note list --status draft`.
3. Decide what to do with each draft. Rewrite it and promote it, merge it into
   another note, or delete it.
4. Promote a draft with `zettel note edit <id> --status permanent`.

A permanent note holds one idea. It links to the related notes. The reviewer
rewrites it in their own words. Do not leave raw agent output in a permanent
note.

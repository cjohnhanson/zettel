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
  -t debugging,postgres -p agent:summary
```

Zettel prints the note ID, for example
`a3f2-connection-pooling-causes-stale-reads`. Most commands accept the
4-character prefix (`a3f2`) instead of the full ID.

The `-p` flag sets the note's default provenance: who produced the text.
An agent passes `agent:summary`, `agent:index`, or `agent:inference`. A
person passes `human` or `human:<name>`. Without the flag the provenance is
unknown, and readers treat unknown text with the most suspicion — always
set it.

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

# Filter by provenance: human, agent, agent:inference, citation,
# reviewed, unknown. A comma list matches any.
zettel note list --provenance human
zettel read --provenance human,citation,reviewed

# List the notes with unreviewed agent content
zettel note list --unreviewed

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

# Delete a note
zettel note delete a3f2-connection-pooling-causes-stale-reads
```

## Mix provenance in one note

A `<!-- prov ... -->` marker overrides the note default for one section:

```bash
zettel note edit a3f2 --append '<!-- prov citation:b7c1 p=12 -->
> The pool serves a stale connection for up to max_age seconds.
<!-- /prov -->

<!-- prov agent:inference -->
The retry storm on 2026-08-02 probably started here.
<!-- /prov -->

<!-- prov human:cody -->
Confirmed with the infra team.
<!-- /prov -->'
```

A citation names its source: another note's ID, or a `src=<url>` attribute
for an external source. Text outside the markers keeps the note default.

## The review workflow

1. Agents create notes and mark spans with their provenance.
2. A human lists the pending work: `zettel note list --unreviewed`.
3. `zettel note review <id>` shows the numbered spans.
4. The human approves what they stand behind:
   `zettel note review <id> --approve all --reviewer <name>`, or
   `--approve 2,4` for single spans. The approval writes a `reviewed=` stamp.
5. A reader pulls trusted content with
   `zettel read --provenance human,citation,reviewed`.

Only a human runs `--approve`. Agents never write `reviewed=` stamps.

## Migrate from the status model

Notes from before the provenance model carry a `status:` key. Convert them
once:

```bash
zettel migrate
```

A `permanent` note becomes `provenance: human`. A `draft` note stays
unknown. The status key is removed either way.

<!-- metadata
title: "What is Zettel?"
description: "Zettelkasten-style knowledge management on frontmattered markdown files in git"
type: explanation
-->

# What is Zettel?

Zettel manages a zettelkasten as markdown files with YAML frontmatter
in git.

## Notes and links

A flat directory of markdown files in `.zettel/`. Each note has
frontmatter for title, status, tags, and links. Links between notes
are declared in frontmatter (`links: [note-id]`) or inline via
`[[note-id]]` in the body. Zettel tracks forward links and backlinks.

Notes have two statuses: **draft** and **permanent**. Drafts are
captured ideas. Permanent notes have been reviewed, reformulated, and
linked by a human. Zettel never promotes notes automatically.

## What it's for

Design rationale, integration quirks, debugging notes, things that
don't belong in code comments and aren't worth a standalone doc page
but matter enough to write down.

Agents create draft notes during work. Humans review and promote them.

## What it isn't

Not a wiki, not a documentation system (that's the bundled docs in
each crate), not a task tracker (that's tisket). Plain files in git.
`cat`, `grep`, and the CLI all work.

The CLI adds frontmatter management, link tracking, graph queries
(backlinks, orphans, neighborhood traversal), and search.

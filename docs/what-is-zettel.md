<!-- metadata
title: "What is Zettel?"
description: "Zettelkasten-style knowledge management on frontmattered markdown files in git"
type: explanation
-->

# What is Zettel?

Zettel is a CLI for managing a zettelkasten as markdown files with YAML
frontmatter, stored in git alongside the code they describe.

## The model

A zettelkasten is a collection of atomic notes — one idea per note, written
in the author's own words — connected by explicit links. Zettel implements
this with a flat directory of markdown files, each carrying frontmatter for
title, status, tags, and links.

Notes have two statuses: **draft** and **permanent**. A draft is a captured
idea that hasn't been processed. A permanent note has been reviewed,
reformulated, and linked by a human. Zettel never promotes notes
automatically — that's a deliberate act.

Links between notes are declared in frontmatter (`links: [note-id]`) or
inline via `[[note-id]]` references in the body. Zettel tracks both
directions: forward links and backlinks.

## What zettel is for

Zettel is a knowledge base for a project. The kind of information that
doesn't belong in code comments, isn't worth a doc page, but matters
enough that losing it hurts — design rationale, integration quirks,
debugging techniques, things learned the hard way.

Agents can create draft notes as they work. Humans review, reformulate,
and promote them to permanent. The knowledge base grows alongside the
codebase.

## What zettel isn't

It's not a wiki. There's no rendering, no web interface, no collaboration
features. It's not a documentation system — that's what the bundled docs
in each crate are for. It's not a task tracker — that's tisket.

Zettel is plain files in git. Read them with `cat`, search them with
`grep`, or use the CLI for structured operations. The CLI adds frontmatter
management, link tracking, and graph queries. The files are the thing.

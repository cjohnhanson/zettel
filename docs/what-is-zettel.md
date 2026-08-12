<!-- metadata
title: "What is Zettel?"
description: "Zettelkasten knowledge management in frontmattered markdown files tracked by git"
type: explanation
-->

# What is Zettel?

Zettel manages a zettelkasten. It keeps the notes as markdown files with YAML
frontmatter. Git tracks the files.

## Notes and links

Zettel keeps all notes in one flat directory, `.zettel/`. Each note has
frontmatter for the title, the status, the tags, and the links. A note declares
a link in its frontmatter (`links: [note-id]`) or inline in its body
(`[[note-id]]`). Zettel tracks forward links and backlinks.

A note has one of two statuses: **draft** or **permanent**. A draft note holds
a captured idea. A human reviews, rewrites, and links a permanent note. Zettel
never promotes a note automatically.

## What it is for

Use Zettel for design rationale, integration problems, and debugging notes.
These things do not belong in code comments. They do not need a separate doc
page. They still matter enough to write down.

Agents create draft notes during work. Humans review the drafts and promote
them.

## What it is not

Zettel is not a wiki. It is not a documentation system, because each crate
bundles its own docs. It is not a task tracker, because tisket does that.
Zettel keeps plain files in git. `cat`, `grep`, and the CLI all read them.

The CLI adds frontmatter management, link tracking, graph queries, and search.
The graph queries show backlinks, orphan notes, and the neighborhood of a note.

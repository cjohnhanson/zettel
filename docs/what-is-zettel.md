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
frontmatter for the title, the provenance, the tags, and the links. A note
declares a link in its frontmatter (`links: [note-id]`) or inline in its body
(`[[note-id]]`). Zettel tracks forward links and backlinks.

## Provenance

Every piece of text has a provenance: who produced it and what kind of claim
it makes.

- `human[:name]` — a person wrote it.
- `agent[:kind]` — an agent wrote it. The kind is `summary` (derived from
  sources), `index` (structural), or `inference` (a new claim not present in
  the sources).
- `citation[:source]` — quoted verbatim from a source. A source that
  resolves to a note ID joins the link graph; any other source is an
  external key.
- No provenance means unknown. Zettel never upgrades unknown to human.

The `provenance:` frontmatter key sets the default for the whole note. A
`<!-- prov ... -->` marker in the body overrides it for one section, so one
note mixes origins. A human approves agent content with
`zettel note review`; the approval adds a `reviewed=` stamp. Only a human
runs the review command.

Provenance is a label, not a lock. A later reader, usually an agent, filters
or weighs text by it: a person's text is ground truth, a citation points at
its source, and an unreviewed inference is one agent's guess.

## Composed stores

A knowledge base can declare other knowledge bases it links into. The
declarations live in `stores.yml`, each under a local alias:

```yaml
format: 2
stores:
  - alias: project
    path: ../project
```

A reference then names the store. Write `[[project:a3f2]]` in a body,
`project:a3f2` in `links:`, or `citation:project:a3f2` in a provenance
marker. A reference with no alias stays local to the note that holds it.

The declarations set the direction. A store links only to the stores
that it declares. No store makes itself a target. A personal knowledge
base declares the repositories that it annotates. A repository does not
declare the personal knowledge base. It cannot: the target does not
exist for the other users who clone the repository. Two repositories can
declare each other, because each one is equally reachable.

Each command runs from one store. It reads that store and the stores
that the store declares. From the personal store, a note in a repository
has the ID `project:a3f2`, and the backlinks include the personal
annotations. From the repository, those annotations do not exist. To
change a note in a dependency store, run the command from that store.

If other users clone your store, declare `shared: true`. `zettel check`
then names every declaration that another clone could not follow, such
as a path outside the repository. It counts each one as a problem and
exits non-zero, so a hook or a continuous-integration step stops on it.
Reads through the alias still work on the declaring machine, because
that machine can resolve the path.

## What it is for

Use Zettel for design rationale, integration problems, and debugging notes.
This material is too long for a code comment and too small for a doc page of
its own. It still matters enough to write down.

Agents create notes during work and label them with their provenance. Humans
review the agent content and approve it.

## What it is not

Zettel is not a wiki, a documentation system, or a task tracker. Each crate
bundles its own docs, and tisket holds the work items. Zettel keeps plain
files in git, and `cat`, `grep`, and the CLI all read them.

The CLI adds frontmatter management, link tracking, graph queries, and search.
The graph queries show backlinks, orphan notes, and the neighborhood of a note.

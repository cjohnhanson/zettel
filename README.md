# zettel

Zettel is a zettelkasten knowledge base for code repositories. It keeps atomic
notes as markdown files in `.zettel/`. Notes link to each other through YAML
frontmatter and inline `[[id]]` references. Git tracks the notes next to the
code.

Zettel solves one problem. During research an agent reads code, traces bugs,
and compares options. That work is usually lost at the end of the session.
Zettel writes the work down as notes labeled with their provenance: who
produced each piece of text, and what kind of claim it makes. A human reviews
the agent content later and approves what they stand behind. The notes are
plain text. Git tracks them. Zettel needs no external service.

## Install

```sh
cargo install --git https://github.com/cjohnhanson/zettel
```

## Usage

```sh
zettel init                                                  # run inside a git repo
zettel note create "Connection pooling causes stale reads" \
  --tag bug,postgres --body "Pool reuses sockets after failover..."
zettel note list --tag postgres
zettel search "stale read"
zettel context a3f2 --depth 2
```

The full command set:

```sh
zettel init                                  # make .zettel/ in a git repo
zettel note create "Title" -t a,b -p agent:summary   # make a note; set tags and provenance
zettel note list [--tag t] [--provenance p]  # list the notes
zettel note list --unreviewed                # list the notes with unreviewed agent content
zettel note show <id>                        # show the full content of one note
zettel note edit <id> --add-link b7c1        # link one note to another
zettel note review <id> [--approve all]      # list provenance spans; approve agent content
zettel note delete <id>                      # remove a note
zettel read [--tag t] [--provenance p]       # show the content of the matching notes/spans
zettel search <pattern>                      # search all notes with a regex
zettel backlinks <id>                        # show the notes that link to this note
zettel context <id> --depth N                # show the notes within N hops
zettel orphans                               # show the notes with no links
zettel store list                            # show this store and the stores it declares
zettel check                                 # check for broken links and invalid provenance
zettel migrate                               # convert pre-provenance notes (status keys)
zettel stats                                 # show counts, tag distribution, and connectivity
zettel docs [topic]                          # show the bundled documentation
```

## How it works

Each note is one markdown file with YAML frontmatter:

```markdown
---
id: a3f2
title: Connection pooling causes stale reads
tags: [bug, postgres]
links: [b7c1]
provenance: agent:summary
---

After failover, the pool reuses sockets bound to the old primary.
See [[b7c1]] for the workaround.

<!-- prov agent:inference -->
The 2026-08-02 retry storm probably started here.
<!-- /prov -->

<!-- prov human:cody -->
Confirmed with the infra team.
<!-- /prov -->
```

Zettel keeps the notes in one flat `.zettel/` directory:

```
.zettel/
  a3f2-connection-pooling-stale-reads.md
  b7c1-workaround-force-new-connection.md
```

Notes connect through the frontmatter `links` field and inline `[[id]]`
references. Zettel walks the note graph. It computes backlinks, finds the
orphan notes, shows the neighborhood of a note, and checks for broken links.

Every piece of text has a provenance. The frontmatter `provenance:` key sets
the note default; a `<!-- prov ... -->` marker overrides it for one section.
The origins:

- `human[:name]` — a person wrote it.
- `agent[:kind]` — an agent wrote it: a `summary` of sources, an `index`,
  or an `inference` (a new claim not present in the sources).
- `citation[:source]` — quoted verbatim; the source is a note ID or a
  `src=<url>` attribute. A citation of a note is a link-graph edge.
- No provenance means unknown. Unknown is never upgraded to human, so an
  agent that forgets the label cannot mint human-authored text.

A human approves agent content with `zettel note review <id> --approve`,
which writes a `reviewed=` stamp. A later reader filters by all of this:
`zettel read --provenance human,citation,reviewed` returns only the text a
human wrote, quoted, or vouched for. The labels are convention, not proof —
the same trust model as the files themselves.

## Composed stores

A knowledge base can link into others. Declare them in `stores.yml`:

```yaml
format: 2
stores:
  - alias: project
    path: ../project                            # this machine
  - alias: handbook
    git: https://example.com/org/handbook       # a git repository
  - alias: archive
    blob: s3://bucket/notes                     # object storage
```

`zettel store sync` fetches the remote stores into a local cache. It is
the only command that reaches the network.

A reference then names the store. Write `[[project:a3f2]]` in a body,
`project:a3f2` in `links:`, or `citation:project:a3f2` in a provenance
marker. A reference with no alias stays local.

The declarations set the direction. A personal knowledge base declares
the repositories that it annotates. A repository does not declare the
personal knowledge base. It cannot: the target does not exist for the
other users who clone the repository. Two repositories can declare each
other, because each one is equally reachable.

Each command runs from one store. It reads that store and the stores
that the store declares. Thus one note has different backlinks in
different stores. Dependency stores are read-only.

## Documentation

- [What is Zettel?](docs/what-is-zettel.md) — the zettelkasten model, the note format, the workflow
- [Getting Started](docs/getting-started.md) — a walkthrough of the first notes
- [CLI Reference](docs/cli-reference.md) — the complete command documentation

Run `zettel docs` to read the same documentation from the binary.

## Related

- [tisket](https://github.com/cjohnhanson/tisket) — file-based issue tracker
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill aggregator
- [belmont](https://github.com/cjohnhanson/belmont) — secrets manager for LLM agents
- [mdstore](https://github.com/cjohnhanson/mdstore) — frontmattered markdown library this is built on
- [codelikecody](https://github.com/cjohnhanson/codelikecody) — workflow engine that bundles these

## License

MIT.

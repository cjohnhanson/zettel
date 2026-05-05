# zettel

A zettelkasten knowledge base for code repositories. Captures atomic notes as
markdown files in `.zettel/`, linked through YAML frontmatter and inline
`[[id]]` references, tracked in git alongside the code.

The problem it solves: when an agent does research — reading code, tracing a
bug, weighing options — that work usually evaporates at the end of the
session. zettel writes it down as draft notes a human can later review and
promote into a permanent, curated knowledge base. Plaintext, git-tracked, no
external service.

## Install

```sh
cargo install --git https://github.com/cjohnhanson/zettel
```

## Usage

```sh
zettel init
zettel note create "Connection pooling causes stale reads" \
  --tag bug --tag postgres --body "Pool reuses sockets after failover..."
zettel note list --tag postgres
zettel search "stale read"
zettel context a3f2 --depth 2
```

Full command surface:

```sh
zettel init                                  # set up .zettel/
zettel note create "Title" --tag x --tag y   # create a draft note (--tag repeatable)
zettel note list [--tag t] [--status s]      # list notes
zettel note show <id>                        # full note content
zettel note edit <id> --add-link b7c1        # link two notes
zettel note delete <id>                      # remove a note
zettel read [--tag t] [--status s]           # dump full content of matching notes
zettel search <pattern>                      # regex search across notes
zettel backlinks <id>                        # notes linking to this one
zettel context <id> --depth N                # neighborhood within N hops
zettel orphans                               # notes with no links in or out
zettel check                                 # verify links, tags, frontmatter
zettel stats                                 # counts, tag distribution, connectivity
zettel docs [topic]                          # bundled documentation
```

## How it works

Each note is one markdown file with YAML frontmatter:

```markdown
---
id: a3f2
title: Connection pooling causes stale reads
tags: [bug, postgres]
links: [b7c1]
status: draft
---

After failover, the pool reuses sockets bound to the old primary.
See [[b7c1]] for the workaround.
```

Notes live in a flat `.zettel/` directory:

```
.zettel/
  a3f2-connection-pooling-stale-reads.md
  b7c1-workaround-force-new-connection.md
```

Notes connect through the frontmatter `links` field and inline `[[id]]`
references. zettel walks the graph to compute backlinks, surface orphans,
show a note's neighborhood, and check for broken links.

Every note has a status: `draft` (working, usually agent-created during
research) or `permanent` (curated, written in the author's own words).
Promotion from draft to permanent is always a human decision — agents
write drafts, humans build the permanent knowledge base.

## Documentation

- [What is Zettel?](docs/what-is-zettel.md) — the zettelkasten model, note format, workflow
- [Getting Started](docs/getting-started.md) — first notes walkthrough
- [CLI Reference](docs/cli-reference.md) — complete command documentation

Bundled docs are also browsable via `zettel docs`.

## Related

- [tisket](https://github.com/cjohnhanson/tisket) — file-based issue tracker
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill aggregator
- [belmont](https://github.com/cjohnhanson/belmont) — secrets manager for LLM agents
- [mdstore](https://github.com/cjohnhanson/mdstore) — frontmattered markdown library this is built on
- [codelikecody](https://github.com/cjohnhanson/codelikecody) — workflow engine that bundles these

## License

MIT.

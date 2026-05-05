# 🗃️ zettel

> Zettelkasten for repos. Atomic notes, linked ideas, plain markdown
> in git.

A flat directory of notes in `.zettel/`, each a markdown file with YAML
frontmatter for tags, links, and status. Agents create draft notes during
research. Humans review and promote them to permanent.

## Install

```sh
cargo install --git https://github.com/cjohnhanson/zettel
```

## How it works

Each note holds one idea. Notes link to each other through frontmatter
`links` fields and inline `[[id]]` references. Zettel tracks forward
links, computes backlinks, detects orphans, and traverses the graph to
show a note's neighborhood.

```
.zettel/
  a3f2-connection-pooling-stale-reads.md
  b7c1-workaround-force-new-connection.md
```

Notes have two statuses: `draft` (working, unreviewed) and `permanent`
(curated, linked, written in the author's own words). Promotion from
draft to permanent is always a human decision.

## Usage

```sh
zettel init                                 # set up .zettel/
zettel note create "Title" --tag x --tag y  # create a draft note
zettel note list [--tag t] [--status s]     # list notes
zettel note show <id>                       # full note content
zettel note edit <id> --add-link b7c1       # link two notes
zettel read [--tag t]                       # dump full content of matching notes
zettel search <pattern>                     # regex search across notes
zettel backlinks <id>                       # notes linking to this one
zettel context <id> --depth 3               # neighborhood within N hops
zettel orphans                              # unlinked notes
zettel check                                # verify links, tags, frontmatter
zettel stats                                # note counts, tag distribution, connectivity
zettel docs [topic]                         # bundled documentation
```

## Documentation

- [What is Zettel?](docs/what-is-zettel.md) — the zettelkasten model, note format, workflow
- [Getting Started](docs/getting-started.md) — first notes walkthrough
- [CLI Reference](docs/cli-reference.md) — complete command documentation

## Related

Part of a loose ecosystem of plaintext, git-tracked, agent-readable
tooling.

- [tisket](https://github.com/cjohnhanson/tisket) — file-based issue tracker
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill aggregator
- [belmont](https://github.com/cjohnhanson/belmont) — secrets manager
- [mdstore](https://github.com/cjohnhanson/mdstore) — frontmattered markdown library this is built on
- [codelikecody](https://github.com/cjohnhanson/codelikecody) — workflow engine that bundles these

## License

MIT.

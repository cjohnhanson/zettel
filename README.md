# zettel

Zettel is a zettelkasten knowledge base for code repositories. It keeps atomic
notes as markdown files in `.zettel/`. Notes link to each other through YAML
frontmatter and inline `[[id]]` references. Git tracks the notes next to the
code.

Zettel solves one problem. During research an agent reads code, traces bugs,
and compares options. That work is usually lost at the end of the session.
Zettel writes the work down as draft notes. A human reviews the drafts later
and promotes them into a permanent knowledge base. The notes are plain text.
Git tracks them. Zettel needs no external service.

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
zettel note create "Title" --tag a,b         # make a draft note; separate tags with commas
zettel note list [--tag t] [--status s]      # list the notes
zettel note show <id>                        # show the full content of one note
zettel note edit <id> --add-link b7c1        # link one note to another
zettel note delete <id>                      # remove a note
zettel read [--tag t] [--status s]           # show the full content of the matching notes
zettel search <pattern>                      # search all notes with a regex
zettel backlinks <id>                        # show the notes that link to this note
zettel context <id> --depth N                # show the notes within N hops
zettel orphans                               # show the notes with no links
zettel check                                 # check the notes for broken links
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
status: draft
---

After failover, the pool reuses sockets bound to the old primary.
See [[b7c1]] for the workaround.
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

Every note has a status. A `draft` note is working material. An agent usually
writes it during research. A `permanent` note is reviewed material, written in
the author's own words. Only a human promotes a note from draft to permanent.
Agents write drafts. Humans build the permanent knowledge base.

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

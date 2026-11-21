# zettel

Zettel is a zettelkasten knowledge base for code repositories. It keeps atomic
notes as markdown files in `.zettel/`. Notes link to each other through YAML
frontmatter and inline `[[id]]` references. Git tracks the notes next to the
code, and no external service holds them.

During research an agent reads code, traces bugs, and compares options. That
work is usually lost at the end of the session. Zettel writes it down as notes
labeled with their provenance: who produced each piece of text, and what kind
of claim it makes. A human reviews the agent content later and approves what
they stand behind.

## Install

The command is `zettel`, whichever package you install.

**[Archives of precompiled binaries are available for macOS and
Linux.](https://github.com/cjohnhanson/zettel/releases)** The Linux
binaries are static executables. Each archive holds the binary, its man
pages, the README and the licence. There is no Windows build.

| Package manager | Package | Command |
| --- | --- | --- |
| [Homebrew](https://brew.sh) | [cjohnhanson/tap/zettel](https://github.com/cjohnhanson/homebrew-tap) | `brew install cjohnhanson/tap/zettel` |
| [Cargo](https://doc.rust-lang.org/cargo/) | [zttl](https://crates.io/crates/zttl) | `cargo install --locked zttl` |
| [uv](https://docs.astral.sh/uv/) | [zttl](https://pypi.org/project/zttl/) | `uv tool install zttl` |
| [npm](https://www.npmjs.com) | [@cjohnhanson/zttl](https://www.npmjs.com/package/@cjohnhanson/zttl) | `npm install -g @cjohnhanson/zttl` |

On Debian or Ubuntu, download the `.deb` from the [releases
page](https://github.com/cjohnhanson/zettel/releases) and install it:

```sh
wget https://github.com/cjohnhanson/zettel/releases/download/v0.3.1/zttl_0.3.1-1_amd64.deb
sudo dpkg -i zttl_0.3.1-1_amd64.deb
```

To run it once without installing anything:

```sh
uvx zttl read
npx @cjohnhanson/zttl read
```

### Building

zettel is written in Rust, so you need a [Rust
installation](https://www.rust-lang.org/) to compile it. zettel compiles
with Rust 1.88 or newer. A C compiler is needed as well, which
`aws-lc-sys` uses for its cryptography.

To build zettel:

```sh
git clone https://github.com/cjohnhanson/zettel
cd zettel
cargo build --release
./target/release/zettel --version
```

## Usage

Run these commands in a repository. `zettel init` makes `zettel.yml`
and `.zettel/`. `note create` prints the new note's ID, and later
commands accept that ID, its first four characters, or its slug:

```console
$ zettel init
initialized a zettel store at [CWD]

$ zettel note create "Connection pooling causes stale reads" --tag bug,postgres -p agent:summary --body "After failover, the pool reuses sockets bound to the old primary."
ab12-connection-pooling-causes-stale-reads

$ zettel note create "Workaround: force a new connection" --tag postgres -p human -l connection-pooling-causes-stale-reads
cd34-workaround-force-a-new-connection

$ zettel note list --tag postgres
ab12-connection-pooling-causes-stale-reads  agent:summary  [bug, postgres]  Connection pooling causes stale reads
cd34-workaround-force-a-new-connection  human  [postgres]  Workaround: force a new connection

$ zettel search "stale read"
ab12-connection-pooling-causes-stale-reads  Connection pooling causes stale reads  (title)

$ zettel context workaround-force-a-new-connection --depth 2
--- cd34-workaround-force-a-new-connection ---
title: Workaround: force a new connection
provenance: human
tags: postgres
links: connection-pooling-causes-stale-reads

--- ab12-connection-pooling-causes-stale-reads ---
title: Connection pooling causes stale reads
provenance: agent:summary
tags: bug, postgres

After failover, the pool reuses sockets bound to the old primary.

```

`zettel --help` lists every command. The [CLI
reference](docs/cli-reference.md) shows the `--help` of each one.

## How it works

Each note is one markdown file with YAML frontmatter. This is the first
note from Usage, with two more sections added in an editor:

```markdown {filename=".zettel/ab12-connection-pooling-causes-stale-reads.md"}
---
title: Connection pooling causes stale reads
tags: [bug, postgres]
links: [workaround-force-a-new-connection]
provenance: agent:summary
---

After failover, the pool reuses sockets bound to the old primary.
See [[workaround-force-a-new-connection]] for the workaround.

<!-- prov agent:inference -->
The 2026-08-02 retry storm probably started here.
<!-- /prov -->

<!-- prov human:cody -->
Confirmed with the infra team.
<!-- /prov -->
```

The note ID is the filename stem, so the frontmatter does not repeat
it. Zettel keeps the notes in one flat `.zettel/` directory:

```console
$ ls .zettel
ab12-connection-pooling-causes-stale-reads.md
cd34-workaround-force-a-new-connection.md

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
human wrote, quoted, or vouched for. Nothing verifies a label against what
actually wrote the span. Reading the files directly gives the same labels
and the same caveat.

`zettel note review` lists the spans of the note above, each with its
provenance:

```console
$ zettel note review connection-pooling-causes-stale-reads
ab12-connection-pooling-causes-stale-reads  Connection pooling causes stale reads

  1  agent:summary (default)  After failover, the pool reuses sockets bound to the old pri…  (+1 more line(s))
  2  agent:inference  The 2026-08-02 retry storm probably started here.
  4  human:cody  Confirmed with the infra team.

```

## Composed stores

A knowledge base can link into others. Declare them in `stores.yml`:

```yaml {filename="stores.yml"}
format: 2
stores:
  - alias: project
    path: ../project                            # this machine
  - alias: handbook
    git: https://example.com/org/handbook       # a git repository
  - alias: archive
    blob: https://example.com/notes             # an https prefix with an index.txt
```

`zettel store list` shows each declared store, or why it is unavailable.
Here `../project` does not exist, and the remote stores wait for
`zettel store sync`:

```console
$ zettel store list
(this store)  [CWD]  2 note(s)
project  ../project  unavailable: no directory at [CWD]/../project
handbook  https://example.com/org/handbook  unavailable: https://example.com/org/handbook is not in the cache; run store sync
archive  https://example.com/notes  unavailable: https://example.com/notes is not in the cache; run store sync

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

Each command runs from one store. Graph queries and note lookups also
read the stores that the store declares. `zettel search` and
`zettel read` use only the current store. Thus one note has different
backlinks in different stores. Dependency stores are read-only.

## Serving over MCP

`zettel serve` offers the same knowledge base to a Model Context
Protocol client. The library answers both interfaces, so the server
returns what the CLI returns.

- `zettel serve` speaks MCP on stdin and stdout, for a client that
  starts the process.
- `zettel serve --bind 127.0.0.1:7431` serves over HTTP at
  `http://127.0.0.1:7431/mcp`, for a client that connects.
- `zettel serve --access read-write` also allows note creation.

The server is read-only by default. With `--access read-write` it
stamps every note it writes as agent-written. It refuses any other
provenance, and it never exposes approval. A served store has no
authentication: bind it to `127.0.0.1`, or put a proxy that
authenticates in front of it.

## Documentation

- [What is Zettel?](docs/what-is-zettel.md) — the zettelkasten model and the workflow
- [Getting started](docs/getting-started.md) — a walkthrough of the first notes
- [How to use project notes with a coding agent](docs/how-to-use-notes-with-an-agent.md)
- [How to review agent notes](docs/how-to-review-agent-notes.md)
- [How to repair broken note links](docs/how-to-repair-broken-links.md)
- [Note format](docs/note-format.md) — every field of a note file
- [CLI reference](docs/cli-reference.md) — the complete command documentation

Run `zettel docs` to read the same documentation from the binary.

## Related

- [tisket](https://github.com/cjohnhanson/tisket) — issue tracker. Markdown issues with YAML frontmatter, in the repository
- [almanac](https://github.com/cjohnhanson/almanac) — agent skill index, over pluggable sources
- [gaff](https://github.com/cjohnhanson/gaff) — context-lifecycle handler for coding agents
- [missouri](https://github.com/cjohnhanson/missouri) — end-to-end tests as directed graphs of filesystem states
- [mdstore](https://github.com/cjohnhanson/mdstore) — the frontmattered markdown library zettel stores notes with

## License

MIT.

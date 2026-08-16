<!-- metadata
title: "zettel CLI Reference"
description: "Complete command reference for the zettel knowledge base"
type: reference
-->

# zettel CLI Reference

```
zettel <command>
```

Zettelkasten note management on frontmattered markdown.

## Global Options

`--root <path>` — The root directory of the repository. The default is `.`, the current directory. This option applies to all subcommands.

`--version` — Print the version and exit.

`--help` — Print the help and exit.

## Commands

### `zettel init`

Initialize Zettel in the current directory. The command makes `zettel.yml` and a `.zettel/` directory.

The command fails if `zettel.yml` already exists.

### `zettel note create <title>`

Create a note. Zettel prints the new note ID to stdout.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tag <csv>` | `-t` | | The tags; separate them with commas |
| `--links <csv>` | `-l` | | The note IDs to link to; separate them with commas |
| `--body <text>` | `-b` | | The note body text |
| `--provenance <spec>` | `-p` | | The default provenance: `origin[:qualifier]`, for example `human:cody`, `agent:summary`, `citation:ab12`. Omitted means unknown |

### `zettel note list`

List the notes. The default lists all notes.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tag <tag>` | `-t` | | Filter by tag |
| `--provenance <tokens>` | `-p` | | Filter by provenance tokens, separated with commas: `human`, `agent`, `agent:inference`, `citation`, `reviewed`, `unknown`. A note matches when any span matches any token |
| `--unreviewed` | | | Keep only the notes with unreviewed agent content |
| `--where <selector>` | | | Filter by selector (`namespace:value`). Repeat the option to add selectors. Zettel combines them with AND. The `provenance` namespace takes the same tokens |
| `--format <fmt>` | | `text` | The output format: `text` or `json` |

Text output columns: `ID`, `PROVENANCE`, `[TAGS]`, `TITLE`.

### `zettel note show <id>`

Show the full details of a note.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | The output format: `text` or `json` |
| `--field <name>` | | | Print one field value |

Valid `--field` values: `title`, `provenance`, `tags`, `links`, `body`, `id`.

The JSON output carries a `spans` array. Each span holds its text, its
resolved provenance, and whether the provenance comes from the note default.

### `zettel note edit <id>`

Edit a note. Zettel changes only the fields you give.

| Option | Short | Description |
|--------|-------|-------------|
| `--title <text>` | | Replace the title |
| `--provenance <spec>` | `-p` | Set the default provenance: `origin[:qualifier]` |
| `--tag <csv>` | `-t` | Replace all tags; separate them with commas |
| `--add-tag <tag>` | | Add one tag and keep the existing tags |
| `--remove-tag <tag>` | | Remove one tag and keep the other tags |
| `--links <csv>` | `-l` | Replace all links; separate them with commas |
| `--add-link <id>` | | Add one link and keep the existing links |
| `--remove-link <id>` | | Remove one link and keep the other links |
| `--body <text>` | | Replace the whole body |
| `--append <text>` | | Append text to the body |

Zettel sets the `updated` timestamp automatically.

### `zettel note delete <id>`

Delete the note file. You cannot undo this.

### `zettel note review <id>`

Without options: list the note's provenance spans, numbered. With
`--approve`: stamp agent spans as human-approved. Only a human runs
`--approve`.

| Option | Description |
|--------|-------------|
| `--approve <spans>` | `all` for every unreviewed agent span, or 1-based span numbers separated with commas |
| `--reviewer <name>` | The reviewer name to record with the approval |

The approval writes `reviewed=<date>` (and `reviewer=<name>`) into the span
marker, or into the note's default provenance spec for unmarked text.
Approving a non-agent span fails. A note with no body spans approves
through its default.

A later `--body` or `--append` edit removes the default's review stamp:
the stamp vouched for text that changed. A stamp on a body marker stays,
because its span text did not change. Setting the same provenance again
with `note edit --provenance` keeps the stamp; a different origin or
qualifier drops it.

---

## Graph Commands

### `zettel backlinks <id>`

Show the notes that link to the given note.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | The output format: `text` or `json` |

### `zettel orphans`

List the notes with no inbound links and no outbound links.

### `zettel context <id>`

Show a note and the linked notes within N hops.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--depth <n>` | `-d` | `2` | The maximum link depth to traverse |
| `--format <fmt>` | | `text` | The output format: `text` or `json` |

---

## Search and Read

### `zettel search <pattern>`

Search the notes with a regex pattern. Zettel matches the frontmatter fields and the body.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | The output format: `text` or `json` |

Text output columns: `ID`, `TITLE`, `(MATCHED_FIELDS)`.

### `zettel read`

Show the full content of the matching notes. Zettel prints a frontmatter summary and the body of each note.

| Option | Short | Description |
|--------|-------|-------------|
| `--tag <tag>` | `-t` | Filter by tag |
| `--provenance <tokens>` | `-p` | Filter by provenance tokens, separated with commas. Only the matching spans print, each under a `[provenance]` label. Notes with no match are omitted |

---

## `zettel stats`

Print the knowledge base statistics: the total note count, the span counts by origin (with the unreviewed agent count), the orphan count, the tag distribution, and the most connected notes.

---

## `zettel store list`

List the stores that this store reads. The first row is the store
itself. Each other row shows a declared dependency, its source, and its
note count. If a dependency is not reachable, the row gives the reason.

Declarations live in `stores.yml` beside `zettel.yml`:

```yaml
format: 2
shared: false          # true when other people clone this store
stores:
  - alias: project
    path: ../project           # a directory on this machine
  - alias: handbook
    git: https://example.com/org/handbook
    rev: v1.0                  # optional; the default branch without it
  - alias: archive
    blob: https://example.com/notes    # an https prefix with an index.txt
```

A shared store declares an outside dependency by URL, because a path
reaches only this machine.

## `zettel store sync`

Fetch each declared remote store into the local cache. This is the only
command that reaches the network. Every other command reads what the
cache already holds, so an answer never changes because of a fetch that
you did not ask for.

A git store keeps one bare clone for each URL, and its notes are read
from git objects at the revision that each store declares. Two stores
that pin different revisions of one URL therefore share one clone. The
clone and the fetch run in-process: https and git:// over gix's own
transports, a local repository by reading its object database. No git
program runs. An ssh URL is refused, because gix would spawn ssh for
it; declare the store with https. A blob store is an https prefix that
publishes an `index.txt`; sync fetches the index and each document by
GET. `s3://` and `gs://` are refused.

`store list` gives the age of each cache, so a stale answer looks stale.

## The registry

A shared store declares a dependency by URL. If the same repository is
already checked out on this machine, bind it in
`~/.config/mdstore/registry.yml`:

```yaml
stores:
  - git: https://example.com/org/handbook
    path: ~/Projects/handbook
```

The registry changes where a dependency resolves. It does not change
what a store declares.

It does change what a command reads. The checkout answers for the
declared source, including for a pinned revision, so a command can read
a note that is only in that working tree. `zettel store list` marks a
row the registry bound, and `zettel check` reports a reference that
resolves only through the checkout: that reference works here and
nowhere else.

A command reads all the declared stores. A command writes only to this
store. A note in a dependency has the ID `alias:id`. A note in a
dependency of a dependency has the ID `alias/alias:id`. The commands
accept that longer form for read operations only.

---

## `zettel check`

Check every note for broken links, invalid provenance, and unparseable
files. The command lists each finding with its note ID and exits non-zero
when it finds any. One corrupt file never breaks the repo-wide commands:
they skip it with a warning, and `check` names it.

If the store declares other stores, `check` reports these conditions
also:

- A reference uses an alias that the store does not declare.
- A declared store is not available.
- A citation key reads as a store reference, but no note has that ID.
- The scan refused to read a file.
- In a `shared: true` store, a clone cannot reach a dependency.

---

## `zettel migrate`

Convert pre-provenance notes. `status: permanent` becomes
`provenance: human`; `status: draft` stays unknown. The status key is
removed either way. A second run changes nothing.

---

## `zettel prime`

Print what zettel is and how to use it, for an agent's context. The
output depends only on the binary version: no arguments, config, or
store changes it. Put it into an agent's context; policy about when to
use zettel belongs to the caller.

---

## `zettel docs`

Read the bundled Zettel documentation.

```
zettel docs                    List the available docs and their slugs
zettel docs list               Do the same as bare `zettel docs`
zettel docs <identifier>       Print a doc by slug, title, or unique prefix
zettel docs search <query>     Search all docs
```
---

## `zettel serve`

Serve this knowledge base over the Model Context Protocol.

```
zettel serve                        Speak MCP on stdin and stdout
zettel serve --root <DIR>           Serve the store at DIR (default: .)
zettel serve --bind <ADDR>          Serve over HTTP at ADDR instead
zettel serve --surfaces <LIST>      Offer these surfaces (default: resources,tools)
zettel serve --access <MODE>        read-only (default) or read-write
```

Omit `--bind`, and the server speaks on stdin and stdout, for a client
that starts the process. Give `--bind`, and it serves over HTTP, for a
client that connects to it. The endpoint is `/mcp` on the bound
address, so `--bind 127.0.0.1:7431` serves at
`http://127.0.0.1:7431/mcp`.

`--surfaces` takes `resources`, `tools`, or both, separated by commas.
The protocol cannot negotiate which of these a client understands, so
the choice is configuration. Tools are the surface every client can
call, so the default keeps them on.

`--access read-write` adds note creation. The server stamps a note it
writes as agent-written, whichever access mode is set. It refuses any
other provenance, and it never exposes approval. Only
`zettel note review --approve` writes a `reviewed=` stamp.

### Authentication

A served store has none. The server answers whoever opens the
connection.

This is deliberate. Authentication belongs in front of the server, in
something built for it: a reverse proxy that terminates TLS and checks
a token or an identity provider.

Bind to `127.0.0.1` for a client on this machine. To serve anybody
else, put the server behind a proxy that authenticates.

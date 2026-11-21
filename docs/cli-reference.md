<!-- metadata
title: "zettel CLI reference"
description: "Every command and option, root resolution, store declarations, and the registry"
type: reference
-->

# zettel CLI reference

Each command section is the output of `zettel <command> --help`. The
sections after them describe how a command finds its store, the
`stores.yml` declarations, and the registry.

<!-- Begin generated CLI help: cargo test --test docs -->

## zettel

```text
Zettelkasten note management on frontmattered markdown

Usage: zettel [OPTIONS] <COMMAND>

Commands:
  init       Initialize Zettel in the current directory
  note       Manage the notes
  backlinks  Show the notes that link to a note
  orphans    Show the notes with no links
  search     Search the notes with a regex pattern
  read       Show the full content of the matching notes
  context    Show a note and the linked notes within N hops
  stats      Show the knowledge base statistics
  store      List, sync, and set the stores that zettel reads
  check      Check the notes and the declared stores for problems
  migrate    Convert pre-provenance notes (status keys) to the provenance model
  serve      Serve this knowledge base over MCP
  docs       Read the bundled documentation
  prime      Print what zettel is and how to use it, for an agent's context
  help       Print this message or the help of the given subcommand(s)

Options:
      --root <ROOT>  Store directory. Literal: the directory must hold zettel.yml; no walk, no
                     fallback. Without it, the nearest zettel.yml at or above the cwd is used.
                     With none, reads use the configured root store and a write needs --home
      --home         Act on the configured root store (`zettel store root`), wherever the command
                     runs
  -h, --help         Print help
  -V, --version      Print version
```

## zettel init

```text
Initialize Zettel in the current directory

Writes zettel.yml, which names .zettel as the note directory, creates .zettel/, and prints the
path of the new store. With --root, it initializes that directory instead. The command fails when
zettel.yml already exists.

Usage: zettel init [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note

```text
Manage the notes

Usage: zettel note [OPTIONS] <COMMAND>

Commands:
  create  Create a note
  list    List the notes
  show    Show a note
  edit    Edit a note
  delete  Delete a note
  review  List the provenance spans of a note, or approve agent spans
  help    Print this message or the help of the given subcommand(s)

Options:
      --root <ROOT>  Store directory. Literal: the directory must hold zettel.yml; no walk, no
                     fallback. Without it, the nearest zettel.yml at or above the cwd is used.
                     With none, reads use the configured root store and a write needs --home
      --home         Act on the configured root store (`zettel store root`), wherever the command
                     runs
  -h, --help         Print help
```

## zettel note create

```text
Create a note

Prints the new note's ID: a random four-character prefix, a hyphen, and a slug of the title, such
as a3f2-my-first-note. A slug longer than 100 bytes is cut at the last word that fits. The command
fails when another note already uses the slug. Later commands accept the full ID, the prefix, or
the slug. Tags and links are stored as written.

Usage: zettel note create [OPTIONS] <TITLE>

Arguments:
  <TITLE>
          The note title

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

  -t, --tag <TAG>
          The tags; separate them with commas

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -l, --links <LINKS>
          The link IDs; separate them with commas

  -b, --body <BODY>
          The note body text

  -p, --provenance <PROVENANCE>
          The default provenance: origin[:qualifier], for example human:cody, agent:summary, or
          citation:ab12. Omitted means unknown

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note list

```text
List the notes

Each line shows the ID, the default provenance (unknown when the note has none), the tags in
brackets when there are any, and the title.

JSON output is an array with one object for each note: id, title, provenance, tags, links,
created, updated, body, and spans. provenance, created, and updated are null when the note has
none. Each span holds its text, its provenance, and from_default, which is true when the
provenance comes from the note default.

Usage: zettel note list [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

  -t, --tag <TAG>
          Filter by tag

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -p, --provenance <PROVENANCE>
          Filter by provenance tokens, separated with commas: human, agent, agent:inference,
          citation, reviewed, unknown. A note matches when any of its spans matches any token

      --unreviewed
          Keep only the notes with at least one unreviewed agent span

      --where <WHERE>
          Filter by selector, namespace:value. The namespaces are tag, provenance (the tokens of
          --provenance), and link. Repeat the option to add selectors; a note must match every one

      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note show

```text
Show a note

Prints the ID, then the title, the default provenance, the tags and links when there are any, and
the body. The note can belong to a store that this store declares; give its ID as alias:id. JSON
output is one object with the fields of a note list object.

Usage: zettel note show [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --field <FIELD>
          Print one field value: title, provenance, tags, links, body, or id

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note edit

```text
Edit a note

Changes only the fields that the options name, and sets the updated timestamp. --tag replaces the
tags before --add-tag and --remove-tag apply, and --links does the same for the links.

A --body or --append edit removes the review stamp from the default provenance, because the stamp
vouched for text that changed. A stamp on a body marker stays. --provenance with the same origin
and qualifier keeps the stamp; a different origin or qualifier drops it.

Usage: zettel note edit [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --title <TITLE>
          The new title

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -p, --provenance <PROVENANCE>
          The new default provenance: origin[:qualifier]

  -t, --tag <TAG>
          Replace all tags

      --add-tag <ADD_TAG>
          Add one tag and keep the existing tags

      --remove-tag <REMOVE_TAG>
          Remove one tag and keep the other tags

  -l, --links <LINKS>
          Replace all links

      --add-link <ADD_LINK>
          Add one link and keep the existing links

      --remove-link <REMOVE_LINK>
          Remove one link and keep the other links

  -b, --body <BODY>
          Replace the whole body

      --append <APPEND>
          Append text to the body

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note delete

```text
Delete a note

Removes the note file and prints "deleted" and the ID as given. Zettel keeps no copy; restore a
deleted note from git.

Usage: zettel note delete [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel note review

```text
List the provenance spans of a note, or approve agent spans

Without --approve, prints each span with its number, its provenance, "(default)" when the
provenance comes from the note default, and the first 60 characters of its first line. The preview
ends in "…" when it cuts the line, and counts the lines it leaves out.

--approve writes reviewed=<date>, and reviewer=<name> with --reviewer, then prints the number of
spans approved. A numbered approval writes them on the span's marker, and first wraps unmarked
text in a new marker. --approve all writes them on the default provenance for unmarked text, and
on each unreviewed agent marker. A number that names a span that is not agent content fails. Only
a human runs --approve.

Usage: zettel note review [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID

Options:
      --approve <APPROVE>
          Approve spans: "all" for every unreviewed agent span, or 1-based span numbers separated
          with commas. Only a human runs this

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

      --reviewer <REVIEWER>
          The reviewer name to record with the approval

  -h, --help
          Print help (see a summary with '-h')
```

## zettel backlinks

```text
Show the notes that link to a note

A note links to another through its frontmatter links or an inline [[id]] in its body. The command
prints the ID and title of each linking note. It also reads the stores that this store declares,
and a note from one of them prints as alias:id. JSON output holds id, title, and store for each
note.

Usage: zettel backlinks [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel orphans

```text
Show the notes with no links

An orphan links to no note, and no note links to it. Each line shows the ID, the default
provenance, the tags, and the title, as note list does.

Usage: zettel orphans [OPTIONS]

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel search

```text
Search the notes with a regex pattern

The pattern is matched against the title, each tag, and the body of every note in this store. The
stores that this store declares are not searched. Each line shows the ID, the title, and the
matched fields in parentheses. JSON output adds matched_fields to each note object of note list.

Usage: zettel search [OPTIONS] <PATTERN>

Arguments:
  <PATTERN>
          The search pattern, a regular expression. It matches case; (?i) at the front turns that
          off

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel read

```text
Show the full content of the matching notes

Reads this store only, not the stores it declares. For each note, the command prints a "--- <id>
---" line, then title, provenance, created when the note has it, tags and links when there are
any, a blank line, and the body.

With --provenance, only the matching spans print, each under a [provenance] label. A note with no
matching span is left out.

Usage: zettel read [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

  -t, --tag <TAG>
          Filter by tag

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -p, --provenance <PROVENANCE>
          Filter by provenance tokens, separated with commas. Only the matching spans print, and
          notes with no match are omitted

  -h, --help
          Print help (see a summary with '-h')
```

## zettel context

```text
Show a note and the linked notes within N hops

Links count in both directions: a note that links to the start note is one hop away, as is a note
the start note links to. The walk also enters the stores that this store declares. For each note,
the command prints a "--- <id> ---" line, then title, provenance, tags and links when there are
any, and the body.

Usage: zettel context [OPTIONS] <ID>

Arguments:
  <ID>
          The note ID to start from

Options:
  -d, --depth <DEPTH>
          The maximum link depth to traverse. The default is 2
          
          [default: 2]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel stats

```text
Show the knowledge base statistics

Prints the note count, the span counts by origin with the count of unreviewed agent spans, the
orphan count, the count of notes for each tag, and up to five notes with the most backlinks.

Usage: zettel stats [OPTIONS]

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel store

```text
List, sync, and set the stores that zettel reads

Usage: zettel store [OPTIONS] <COMMAND>

Commands:
  list  List this store and the stores it declares
  sync  Fetch the declared remote stores into the local cache
  root  Show or set the root store that reads fall back to
  help  Print this message or the help of the given subcommand(s)

Options:
      --root <ROOT>  Store directory. Literal: the directory must hold zettel.yml; no walk, no
                     fallback. Without it, the nearest zettel.yml at or above the cwd is used.
                     With none, reads use the configured root store and a write needs --home
      --home         Act on the configured root store (`zettel store root`), wherever the command
                     runs
  -h, --help         Print help
```

## zettel store list

```text
List this store and the stores it declares

The first row is this store. Each other row shows a declared store's alias, its source, and its
note count, or why it is not available. A remote store's row also shows the age of its cache, and
a row that the registry binds to a local checkout names that checkout.

Usage: zettel store list [OPTIONS]

Options:
      --format <FORMAT>
          The output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel store sync

```text
Fetch the declared remote stores into the local cache

This is the only command that reaches the network. Every other command reads what the cache
already holds.

Each store syncs on its own, and one failure never stops another. The command prints a line for
each store, then exits 1 if any failed. A store fails when its source is unreachable, or when its
declared revision is absent from what arrived. With no remote store declared, it prints "no remote
stores declared".

A git store keeps one bare clone for each URL, and reads its notes at the revision that each
declaration names. The fetch runs in-process over https or git://, or from a local repository's
object database, and no git program runs. An ssh URL is refused; declare the store with https. A
blob store is an https prefix that publishes an index.txt; sync fetches the index and each
document it names. s3:// and gs:// are refused.

Usage: zettel store sync [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel store root

```text
Show or set the root store that reads fall back to

Without a path, prints the current setting and the file it came from. With a path, writes it to
~/.config/zettel/config.yml; the directory must hold zettel.yml. Changing a setting that is
already there needs --force. Each tool reads its own config file, and this command writes only the
file that zettel reads.

Usage: zettel store root [OPTIONS] [PATH]

Arguments:
  [PATH]
          The directory of the root store. Without it, print the current setting

Options:
      --force
          Replace an already-set root with a different one

      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel check

```text
Check the notes and the declared stores for problems

Prints "no problems found", or a count and one line for each finding: the source, its title, the
target, and where the problem is. Exits 1 when it finds any. The location is one of:

frontmatter: a links entry names no note, or the file does not parse

body: an inline [[id]] names no note

provenance: a provenance spec or marker is invalid

declaration: another clone of a shared store cannot reach a declared store, or a declared store
cannot be read

unreachable store: a declared store is not available

shadowed citation: a citation key now reads as a reference to a declared store

override: a reference resolves only through a checkout that the registry names

scan: the scan refused to read a file

Other commands skip a file that does not parse; check names it.

Usage: zettel check [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel migrate

```text
Convert pre-provenance notes (status keys) to the provenance model

Removes the status key from every note that has one. A note with "status: permanent" and no
provenance gets "provenance: human". Any other note keeps its provenance, so a draft stays
unknown. Prints one line for each changed note, or "nothing to migrate". A second run changes
nothing.

Usage: zettel migrate [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

## zettel serve

```text
Serve this knowledge base over MCP

Without --bind, the server speaks the Model Context Protocol on stdin and stdout, for a client
that starts the process. With --bind, it serves over HTTP at /mcp on that address, for a client
that connects: --bind 127.0.0.1:7431 serves http://127.0.0.1:7431/mcp.

The server stamps every note it writes as agent-written, refuses any other provenance, and never
exposes approval.

A served store has no authentication: the server answers whoever connects. Bind to 127.0.0.1 for a
client on this machine. To serve anyone else, put a reverse proxy that authenticates in front of
the server.

Usage: zettel serve [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --surfaces <SURFACES>
          The surfaces to offer, separated by commas: resources, tools. Tools are the surface
          every client can call, so a default keeps them on
          
          [default: resources,tools]

      --access <ACCESS>
          What a caller may do: read-only or read-write. A note written through the server is
          recorded as agent-written whichever is set
          
          [default: read-only]

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

      --bind <BIND>
          Where to listen. Omitted, the server speaks on stdin and stdout, for a client that
          starts it. Given, it serves over HTTP for a client that connects

  -h, --help
          Print help (see a summary with '-h')
```

## zettel docs

```text
Read the bundled documentation

Usage: zettel docs [OPTIONS] [TOPIC] [QUERY]

Arguments:
  [TOPIC]  Topic slug to show, or "search" to search the docs
  [QUERY]  Search query (when topic is "search")

Options:
  -a, --all          Print every page whole, in one stream
      --root <ROOT>  Store directory. Literal: the directory must hold zettel.yml; no walk, no
                     fallback. Without it, the nearest zettel.yml at or above the cwd is used.
                     With none, reads use the configured root store and a write needs --home
      --home         Act on the configured root store (`zettel store root`), wherever the command
                     runs
  -h, --help         Print help
```

## zettel prime

```text
Print what zettel is and how to use it, for an agent's context

The output depends only on the binary version: no argument, config, or store changes it. It says
nothing about when to use zettel; that policy belongs to the caller.

Usage: zettel prime [OPTIONS]

Options:
      --root <ROOT>
          Store directory. Literal: the directory must hold zettel.yml; no walk, no fallback.
          Without it, the nearest zettel.yml at or above the cwd is used. With none, reads use the
          configured root store and a write needs --home

      --home
          Act on the configured root store (`zettel store root`), wherever the command runs

  -h, --help
          Print help (see a summary with '-h')
```

<!-- End generated CLI help -->

## Root resolution

Without `--root`, a command finds its store by one rule, identical in
tisket and almanac: the nearest `zettel.yml` at or above the working
directory wins. The walk requires a regular file. It stops at the first
directory the invoking user does not own, so a marker planted in a
shared ancestor captures nothing. With no store found, a read falls
back to the root store set in `~/.config/zettel/config.yml` and says
so on stderr. A write never falls back. It fails and names `--home`.
No environment variable participates: a variable is the one input an
agent cannot see in a transcript, and a repository can set one through
direnv or mise. The config path itself is fixed, and the home directory
comes from the passwd database, not `$HOME`, for the same reason.

With `--root`, the command uses that directory and reads no user
config. `zettel init`, `zettel prime`, `zettel docs`, and
`zettel store root` resolve no store.

A write prints its resolved target on stderr unless the target is the
working directory's own store or `--root` named it. A read resolved by
walking or by fallback prints its source.

## Store declarations

`zettel store list` reads the declarations in `stores.yml` beside
`zettel.yml`:

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

Note lookups and graph queries can read declared stores. `search` and
`read` operate on the current store only; run them from a dependency's
root, or select that root with `--root`, to search or read its notes.
A command writes only to this store. A note in a dependency has the ID
`alias:id`. A note in a dependency of a dependency has the ID
`alias/alias:id`. The commands accept that longer form for read
operations only.

## The registry

A shared store declares a dependency by URL. If the same repository is
already checked out on this machine, bind it in
`~/.config/zettel/registry.yml`:

```yaml
stores:
  - git: https://example.com/org/handbook
    path: ~/Projects/handbook
```

The registry file is `zettel/registry.yml` under `XDG_CONFIG_HOME` when
that variable is set. `MDSTORE_REGISTRY` names the file outright.

A store's declarations stay as written. The registry redirects where
each declared source resolves, so the checkout answers for that source,
pinned revision included. A command can then read a note that is only
in that working tree. `zettel store list` marks a
row the registry bound, and `zettel check` reports a reference that
resolves only through the checkout: that reference works here and
nowhere else.

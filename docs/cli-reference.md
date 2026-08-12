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
| `--tags <csv>` | `-t` | | The tags; separate them with commas |
| `--links <csv>` | `-l` | | The note IDs to link to; separate them with commas |
| `--body <text>` | `-b` | | The note body text |
| `--status <status>` | `-s` | `draft` | The initial status: `draft` or `permanent` |

### `zettel note list`

List the notes. The default lists all notes.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tag <tag>` | `-t` | | Filter by tag |
| `--status <status>` | `-s` | | Filter by status: `draft` or `permanent` |
| `--where <selector>` | | | Filter by selector (`namespace:value`). Repeat the option to add selectors. Zettel combines them with AND |
| `--format <fmt>` | | `text` | The output format: `text` or `json` |

Text output columns: `ID`, `STATUS`, `[TAGS]`, `TITLE`.

### `zettel note show <id>`

Show the full details of a note.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | The output format: `text` or `json` |
| `--field <name>` | | | Print one field value |

Valid `--field` values: `title`, `status`, `tags`, `links`, `body`, `id`.

### `zettel note edit <id>`

Edit a note. Zettel changes only the fields you give.

| Option | Short | Description |
|--------|-------|-------------|
| `--title <text>` | | Replace the title |
| `--status <status>` | `-s` | Set the status: `draft` or `permanent` |
| `--tags <csv>` | `-t` | Replace all tags; separate them with commas |
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
| `--status <status>` | `-s` | Filter by status: `draft` or `permanent` |

---

## `zettel stats`

Print the knowledge base statistics: the total note count, the draft and permanent counts, the orphan count, the tag distribution, and the most connected notes.

---

## `zettel docs`

Read the bundled Zettel documentation.

```
zettel docs                    List the available docs and their slugs
zettel docs list               Do the same as bare `zettel docs`
zettel docs <identifier>       Print a doc by slug, title, or unique prefix
zettel docs search <query>     Search all docs
```

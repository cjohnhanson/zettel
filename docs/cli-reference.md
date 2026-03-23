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

`--root <path>` — Root directory of the repository. Defaults to `.` (current directory). Applies to all subcommands.

`--version` — Print version and exit.

`--help` — Print help and exit.

## Commands

### `zettel init`

Initialize zettel in the current directory. Creates `zettel.yml` and a `.zettel/` directory.

Fails if `zettel.yml` already exists.

### `zettel note create <title>`

Create a new note. Prints the generated note ID to stdout.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tags <csv>` | `-t` | | Comma-separated tags |
| `--links <csv>` | `-l` | | Comma-separated note IDs to link to |
| `--body <text>` | `-b` | | Note body text, inline |
| `--status <status>` | `-s` | `draft` | Initial status: `draft` or `permanent` |

### `zettel note list`

List notes. By default, lists all notes.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tag <tag>` | `-t` | | Filter by tag |
| `--status <status>` | `-s` | | Filter by status: `draft` or `permanent` |
| `--where <selector>` | | | Filter by selector (`namespace:value`). Repeatable; multiple selectors AND together |
| `--format <fmt>` | | `text` | Output format: `text` or `json` |

Text output columns: `ID`, `STATUS`, `[TAGS]`, `TITLE`.

### `zettel note show <id>`

Show full details for a note.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | Output format: `text` or `json` |
| `--field <name>` | | | Extract a single field value |

Valid `--field` values: `title`, `status`, `tags`, `links`, `body`, `id`.

### `zettel note edit <id>`

Edit an existing note. Only specified options are changed.

| Option | Short | Description |
|--------|-------|-------------|
| `--title <text>` | | Replace the title |
| `--status <status>` | `-s` | Set status: `draft` or `permanent` |
| `--tags <csv>` | `-t` | Replace all tags (comma-separated) |
| `--add-tag <tag>` | | Add a single tag, keeping existing ones |
| `--remove-tag <tag>` | | Remove a single tag, keeping others |
| `--links <csv>` | `-l` | Replace all links (comma-separated) |
| `--add-link <id>` | | Add a link, keeping existing ones |
| `--remove-link <id>` | | Remove a link, keeping others |
| `--body <text>` | | Replace the entire body |
| `--append <text>` | | Append text to the body |

Updates the `updated` timestamp automatically.

### `zettel note delete <id>`

Delete a note file permanently.

---

## Graph Commands

### `zettel backlinks <id>`

Show notes that link to the given note.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | Output format: `text` or `json` |

### `zettel orphans`

List notes with no inbound or outbound links.

### `zettel context <id>`

Show a note and its neighborhood — linked notes within N hops.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--depth <n>` | `-d` | `2` | Maximum link depth to traverse |
| `--format <fmt>` | | `text` | Output format: `text` or `json` |

---

## Search and Read

### `zettel search <pattern>`

Search notes by regex pattern. Matches against frontmatter fields and body.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--format <fmt>` | | `text` | Output format: `text` or `json` |

Text output columns: `ID`, `TITLE`, `(MATCHED_FIELDS)`.

### `zettel read`

Dump full content of matching notes. Prints frontmatter summary and body for each note.

| Option | Short | Description |
|--------|-------|-------------|
| `--tag <tag>` | `-t` | Filter by tag |
| `--status <status>` | `-s` | Filter by status: `draft` or `permanent` |

---

## `zettel stats`

Print knowledge base health: total notes, draft/permanent counts, orphan count, tag distribution, and most-connected notes.

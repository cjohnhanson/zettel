<!-- metadata
title: "Zettel note format"
description: "Note files, frontmatter, provenance, body markers, links, and review stamps."
type: reference
-->

# Zettel note format

This page describes the zettel note file format.

## File name and ID

Each note is one Markdown file in a flat directory. `zettel_dir` in `zettel.yml` names that directory, and the default is `.zettel`. The file name is `<id>.md`.

An ID contains a 4-character prefix, a hyphen, and a slug of the title. The prefix uses lowercase letters and digits. An example ID is `uith-retries-stop-on-all-errors`.

Commands accept a full ID or the 4-character prefix.

## Frontmatter

Two `---` fences enclose the frontmatter.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `title` | String | Yes | Note title. |
| `provenance` | String | No | Default provenance for unmarked body text. Without this field, that text has unknown provenance. |
| `tags` | List of strings | No | Note tags. The default is an empty list. |
| `links` | List of note IDs or 4-character prefixes | No | Forward links. The default is an empty list. |
| `created` | UTC timestamp | No | Creation time. Zettel fills a missing value when it writes the file. |
| `updated` | UTC timestamp | No | Update time. Zettel fills a missing value when it writes the file. Zettel sets this value on an edit. |

A UTC timestamp has the form `2026-11-07T20:17:49Z`.

When no provenance is given, Zettel writes no `provenance` key.

Edits preserve other keys unchanged. For example, `zettel note edit` preserves `owner: alpha`.

Zettel preserves a `status` key as an unknown key until `zettel migrate` converts it.

## Provenance

A provenance spec has the form `origin[:qualifier] [key=value ...]`.

| Origin | Qualifier | Example |
| --- | --- | --- |
| `human` | None | `human` |
| `human` | Author name | `human:alex` |
| `agent` | None | `agent` |
| `agent` | `summary`: text derived from sources | `agent:summary` |
| `agent` | `index`: structural text | `agent:index` |
| `agent` | `inference`: a new claim absent from the sources | `agent:inference` |
| `citation` | Source note ID or short key for verbatim quoted text | `citation:rfc9110 src=https://example.com/rfc9110` |

Text without provenance has unknown provenance. Zettel never treats unknown provenance as human provenance.

The `src=` attribute holds a source URL. A citation source that resolves to a note ID joins the link graph.

Zettel rejects other agent qualifiers:

```text
error: invalid provenance: 'guess' is not an agent kind; use summary, index, or inference
```

Zettel rejects other origins:

```text
error: invalid provenance: 'robot' is not an origin; use human, agent, or citation
```

## Body markers

A `<!-- prov <spec> -->` line opens a span. A `<!-- /prov -->` line closes it.

Each marker occupies its own line. The parser does not recognize a marker with other text on the same line.

Text outside markers takes the frontmatter default. A marked span has its own provenance.

```markdown
<!-- prov agent:summary -->
Correction recorded in [[cwt7-retry-behavior-differs-by-response]]. Read that note before reusing this finding.
<!-- /prov -->
```

## Links

The frontmatter `links` list declares forward links.

A body reference of the form `[[note-id]]` creates an inline link. This link appears in backlinks.

A reference to a note in a declared store has the form `[[alias:id]]`. A note in a dependency of a dependency has the ID `alias/alias:id`.

`stores.yml` declares other stores. `zettel search` and `zettel read` read only the current store. `zettel context` and `zettel note show alias:id` can read a declared store.

## Review stamps

The attribute `reviewed=<date>` records human approval of an agent span. `reviewer=<name>` follows it when the command receives `--reviewer`. Only `zettel note review --approve` writes these attributes. It writes them on the span's marker, or on the frontmatter `provenance` value when the whole note has one default span.

The command `zettel note review <id> --approve 1 --reviewer alex` produces this file for the example note:

```markdown
---
title: Retries stop on all errors
provenance: agent:inference
tags:
- retry
- superseded
links:
- cwt7-retry-behavior-differs-by-response
created: 2026-11-07T20:17:49Z
updated: 2026-11-07T20:17:49Z
---

<!-- prov agent:inference reviewed=2026-11-07 reviewer=alex -->
The client stops after any failed response.

<!-- /prov -->
<!-- prov agent:summary -->
Correction recorded in [[cwt7-retry-behavior-differs-by-response]]. Read that note before reusing this finding.
<!-- /prov -->
```

The first span has human approval. Its provenance remains `agent:inference`. The second span remains unreviewed.

Zettel rejects a review stamp in a `-p` spec:

```text
error: invalid provenance: a review stamp comes from 'note review --approve', not from a written spec
```

Zettel rejects a review stamp in body text passed to a command:

```text
error: invalid provenance: a review stamp comes from 'note review --approve', not from a written marker
```

`zettel read --provenance human,citation,reviewed` includes human text, citations, and approved agent spans. It excludes unreviewed agent spans.

`zettel note list --unreviewed` lists notes that contain unreviewed agent content.

[How to review agent notes](how-to-review-agent-notes.md) · [the CLI reference](cli-reference.md)

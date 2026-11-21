<!-- metadata
title: "How to repair broken note links"
description: "Locate unresolved references, distinguish missing targets from missing stores, and verify a repair."
type: how-to
-->

# How to repair broken note links

Run this workflow when `zettel check` exits with problems or a linked note is missing from a context query. Start in the intended repository. Use `zettel --root /path/to/repository check` if you need to name the store explicitly.

## Locate the reference

The examples on this page use a store where the note `use-the-shared-http-client` links to `retry-policy`, a note that no longer exists:

```console
$ zettel check
? 1
1 problem(s):
  cd34-use-the-shared-http-client (Use the shared HTTP client) → retry-policy [frontmatter]

```

The first ID names the source note, and its title follows in parentheses. `retry-policy` is its unresolved target. The final field identifies the reference location. The command exits nonzero while problems remain.

Read the source and list available targets. Use the source ID from your own output:

```console
$ zettel note show use-the-shared-http-client
cd34-use-the-shared-http-client

  Title:        Use the shared HTTP client
  Provenance:   human
  Tags:         http
  Links:        retry-policy

The shared client applies the retry rules.

$ zettel note list
ab12-retry-with-backoff  human  [retry]  Retry with backoff
cd34-use-the-shared-http-client  human  [http]  Use the shared HTTP client

```

Compare the target with the actual note IDs. If a note was deleted accidentally, restore its file from the appropriate git revision. If it was replaced, read the replacement before redirecting the link. Do not create an empty note only to make the check pass.

## Replace a frontmatter target

Read the replacement, then remove the incorrect target and add the replacement in one edit:

```console
$ zettel note show retry-with-backoff
ab12-retry-with-backoff

  Title:        Retry with backoff
  Provenance:   human
  Tags:         retry

Retry a 503 response with exponential backoff.

$ zettel note edit use-the-shared-http-client --remove-link retry-policy --add-link retry-with-backoff

$ zettel note show use-the-shared-http-client
cd34-use-the-shared-http-client

  Title:        Use the shared HTTP client
  Provenance:   human
  Tags:         http
  Links:        retry-with-backoff

The shared client applies the retry rules.

```

The edit keeps other links and replaces this one. For an obsolete link with no replacement, use `--remove-link` alone after checking that removing it preserves the note's meaning.

An inline `[[target]]` reference lives in the Markdown body. Correct that reference in the note file; `--remove-link` edits only the frontmatter links. Preserve surrounding evidence and provenance markers.

## Check store availability

If the target contains a store alias, or the finding reports an unavailable store, inspect the declarations:

```console
$ zettel store list
(this store)  [CWD]  2 note(s)

```

Verify the alias in `stores.yml` and the declared path. For a remote store, run `zettel store sync`. A missing store can make valid targets unavailable. Repair the declaration or restore access before changing note IDs. The [composed-store explanation](what-is-zettel.md#composed-stores) describes the direction of these links.

## Verify the repair

```console
$ zettel check
no problems found

$ zettel context use-the-shared-http-client --depth 1
--- cd34-use-the-shared-http-client ---
title: Use the shared HTTP client
provenance: human
tags: http
links: retry-with-backoff

The shared client applies the retry rules.

--- ab12-retry-with-backoff ---
title: Retry with backoff
provenance: human
tags: retry

Retry a 503 response with exponential backoff.

```

A successful check prints `no problems found`. Confirm that the context query includes the intended target. Review the file diff and commit the repair with the work that changed the relationship.

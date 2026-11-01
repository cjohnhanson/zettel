<!-- metadata
title: "How to repair broken note links"
description: "Locate unresolved references, distinguish missing targets from missing stores, and verify a repair."
type: how-to
-->

# How to repair broken note links

Run this workflow when `zettel check` exits with problems or a linked note is missing from a context query. Start in the intended repository. Use `zettel --root /path/to/repository check` if you need to name the store explicitly.

## Locate the reference

```sh
zettel check
```

A broken frontmatter link produces output like this:

```text
1 problem(s):
  b7c1-decision (Decision) → missing-note [frontmatter]
```

The first ID names the source note. `missing-note` is its unresolved target. The final field identifies the reference location. The command exits nonzero while problems remain.

Read the source and list available targets, using the source ID from your output:

```sh
source_note='b7c1-decision'
zettel note show "$source_note"
zettel note list
```

Compare the target with the actual note IDs. If a note was deleted accidentally, restore its file from the appropriate git revision. If it was replaced, read the replacement before redirecting the link. Do not create an empty note only to make the check pass.

## Replace a frontmatter target

Set the incorrect target and the actual replacement ID:

```sh
bad_target='missing-note'
correct_target='a3f2-existing-note-id'
zettel note show "$correct_target"
zettel note edit "$source_note" --remove-link "$bad_target" --add-link "$correct_target"
zettel note show "$source_note"
```

The edit keeps other links and replaces this one. For an obsolete link with no replacement, use `--remove-link` alone after checking that removing it preserves the note's meaning.

An inline `[[target]]` reference lives in the Markdown body. Correct that reference in the note file; `--remove-link` edits only the frontmatter links. Preserve surrounding evidence and provenance markers.

## Check store availability

If the target contains a store alias, or the finding reports an unavailable store, inspect the declarations:

```sh
zettel store list
```

Verify the alias in `stores.yml`, the declared path, and any required remote cache. A missing store can make valid targets unavailable. Repair the declaration or restore access before changing note IDs. The [composed-store explanation](what-is-zettel.md#composed-stores) describes the direction of these links.

## Verify the repair

```sh
zettel check
zettel context "$source_note" --depth 1
```

A successful check prints `no problems found`. Confirm that the context query includes the intended target. Review the file diff and commit the repair with the work that changed the relationship.

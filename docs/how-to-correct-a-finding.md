<!-- metadata
title: "How to correct a finding without losing its evidence"
description: "Record a replacement finding, connect it to the old note, and update dependent notes without erasing history."
type: how-to
-->

# How to correct a finding without losing its evidence

Use a new note when evidence overturns a finding or changes its scope. Keep the earlier note available so a reader can understand the earlier decision. Zettel has no automatic supersession state; the links and tags below are an explicit recording convention.

Start in a repository containing `zettel.yml`. Select the actual ID of the old note:

```sh
old_note='a3f2-your-note-id'
zettel note show "$old_note"
zettel backlinks "$old_note"
```

Inspect the original evidence and the notes that depend on it. Do not replace the old body with a new conclusion.

## Record the correction

Create a note with the corrected claim and its supporting evidence. This runnable example uses synthetic test names; replace the title and body with your actual result:

```sh
new_note=$(zettel note create "Retry behavior differs by response" \
  -t correction -p agent:summary -l "$old_note" \
  -b 'Correction to the linked finding: the synthetic retry_503 test retries a 503 response; retry_400 stops after a 400 response. The earlier statement applied one behavior to both cases.')
printf '%s\n' "$new_note"
```

An agent summary keeps `agent:summary` provenance and requires human review before a reader treats it as reviewed. Include the test command and result or a source citation in a real correction. [Reviewing agent notes](how-to-review-agent-notes.md) covers that step.

Connect the old note to its replacement without erasing its evidence:

```sh
zettel note edit "$old_note" --add-link "$new_note" --add-tag superseded
zettel note edit "$old_note" --append "<!-- prov agent:summary -->
Correction recorded in [[$new_note]]. Read that note before reusing this finding.
<!-- /prov -->"
```

The explicit marker gives the appended correction its own provenance even if the old note has a different default.

## Check downstream use

```sh
zettel note show "$old_note"
zettel context "$new_note" --depth 1
zettel backlinks "$old_note"
zettel check
```

The connected view includes both findings. Inspect each dependent note and update its conclusion or link when it relies on the superseded claim. Keep any relevant evidence in that note too.

The `superseded` tag does not exclude a note from reads, searches, or provenance filters. The warning and replacement link tell a reader which claim to use. If you build an automated reader, make its treatment of that tag explicit.

Review the diff for both notes and commit them together. The Markdown records the correction relationship, and git preserves the earlier versions.

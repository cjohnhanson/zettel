<!-- metadata
title: "How to review agent notes"
description: "Inspect agent spans, approve content a human has checked, and correct a finding."
type: how-to
-->

# How to review agent notes

Use this workflow when an agent has recorded findings that other sessions will reuse. Start in the repository containing `zettel.yml`. [Getting started](getting-started.md) creates a practice store with an unreviewed inference.

## Inspect pending content

```sh
zettel note list --unreviewed
```

Choose a note from the output and assign its actual ID:

```sh
review_note='a3f2-your-note-id'
zettel note show "$review_note"
zettel note review "$review_note"
```

The show command prints the full body. The review command numbers its provenance spans and prints previews. A preview can end in `…`; it is not the complete evidence for approval. A person or agent can run both read commands.

Follow the note's citations and links. Compare summaries with their sources. Check an inference against the relevant code, test, or observation. Leave unsupported claims unapproved. When evidence contradicts a note, [correct the finding](#correct-a-finding).

## Approve the checked spans

Only a human runs the commands in this section. Replace `your-name` with your reviewer name. Use span numbers from the current review output; editing a note can change them.

To approve one checked span, for example span 1:

```sh
zettel note review "$review_note" --approve 1 --reviewer your-name
```

To approve every remaining unreviewed agent span after checking each one:

```sh
zettel note review "$review_note" --approve all --reviewer your-name
```

The command reports the number of approved spans and records review metadata. Approval retains the agent provenance. It does not relabel agent text as human-authored text.

If a numbered approval fails, rerun the review listing. The number must identify an agent span in the current body. A human or citation span is not an agent span to approve.

## Verify the reusable content

```sh
zettel note show "$review_note"
zettel note list --unreviewed
zettel read --provenance human,citation,reviewed
zettel check
```

The full note contains the review metadata. A partially approved note remains in the unreviewed list while it has unreviewed agent content. The filtered read includes approved content and excludes unreviewed agent spans. `zettel check` reports structural problems; it does not judge the claims.

Review the Markdown diff and commit the approved note with the relevant work. A later session can read its evidence and review state from the repository.

## Correct a finding

When evidence overturns a finding or changes its scope, create a correction note linked to the old note.

1. Replace `a3f2-your-note-id` with the old note's ID. Read the note and its backlinks.

   ```sh
   old_note='a3f2-your-note-id'
   zettel note show "$old_note"
   zettel backlinks "$old_note"
   ```

2. Inspect each dependent note. Identify each conclusion that relies on the old claim.

3. Create the correction note. The example uses synthetic test names. Replace the title and body with the real result. Include the test command and result, or a source citation. The example labels agent text. When a person writes the correction, replace `agent:summary` with `human:your-name` in this step and the next.

   ```sh
   new_note=$(zettel note create "Retry behavior differs by response" \
     -t correction -p agent:summary -l "$old_note" \
     -b 'Correction to the linked finding: the synthetic retry_503 test retries a 503 response; retry_400 stops after a 400 response. The earlier statement applied one behavior to both cases.')
   printf '%s\n' "$new_note"
   ```

4. Link the old note to the correction. Keep the old body. Append a notice with its own provenance.

   ```sh
   zettel note edit "$old_note" --add-link "$new_note" --add-tag superseded
   zettel note edit "$old_note" --append "<!-- prov agent:summary -->
   Correction recorded in [[$new_note]]. Read that note before reusing this finding.
   <!-- /prov -->"
   ```

   The appended span has `agent:summary` provenance regardless of the note's default. Agent summaries remain unreviewed until a human approves them. After `--approve all`, an append removes the frontmatter review stamp, so the earlier unmarked text returns to the unreviewed list. A stamp on a body marker stays.

   Zettel has no supersession state. The `superseded` tag is a convention. Reads, searches, and provenance filters do not treat it specially. A program that reads notes must define how it handles this tag.

5. Update each dependent conclusion to reflect the evidence.

6. Check both notes and their links.

   ```sh
   zettel note show "$old_note"
   zettel context "$new_note" --depth 1
   zettel backlinks "$old_note"
   zettel check
   ```

   The context output includes both notes. The backlinks output for the old note includes the correction. `zettel check` reports structural problems. It does not judge claims.

7. Review the diff. Commit both notes together with any dependent note changes. Git keeps earlier versions.

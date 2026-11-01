<!-- metadata
title: "How to review agent notes"
description: "Inspect the full evidence for agent spans and approve only the claims a human has checked."
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

Follow the note's citations and links. Compare summaries with their sources. Check an inference against the relevant code, test, or observation. Leave unsupported claims unapproved. Use [the correction workflow](how-to-correct-a-finding.md) when evidence contradicts a note.

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

Review the Markdown diff and commit the approved note with the relevant work. A later session can read its evidence and review state from the repository. If the evidence changes, preserve the old finding and record a linked correction.

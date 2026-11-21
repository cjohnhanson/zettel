<!-- metadata
title: "How to review agent notes"
description: "Inspect agent spans, approve content a human has checked, and correct a finding."
type: how-to
-->

# How to review agent notes

Use this workflow when an agent has recorded findings that other sessions will reuse. Start in the repository containing `zettel.yml`. [Getting started](getting-started.md) creates a practice store with an unreviewed inference.

## Inspect pending content

```console
$ zettel note list --unreviewed
ab12-retries-stop-on-all-errors  agent:inference  [retry]  Retries stop on all errors

```

Choose a note from the output. The examples on this page review `retries-stop-on-all-errors`; use an ID from your own output:

```console
$ zettel note show retries-stop-on-all-errors
ab12-retries-stop-on-all-errors

  Title:        Retries stop on all errors
  Provenance:   agent:inference
  Tags:         retry

The client stops after any failed response.

<!-- prov agent:summary src=tests/retry.rs -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->

$ zettel note review retries-stop-on-all-errors
ab12-retries-stop-on-all-errors  Retries stop on all errors

  1  agent:inference (default)  The client stops after any failed response.
  2  agent:summary src=tests/retry.rs  The retry tests send a 503 response and a 400 response.

```

The show command prints the full body. The review command numbers its provenance spans and prints previews. A preview can end in `…`; it is not the complete evidence for approval. A person or agent can run both read commands.

Follow the note's citations and links. Compare summaries with their sources. Check an inference against the relevant code, test, or observation. Leave unsupported claims unapproved. When evidence contradicts a note, [correct the finding](#correct-a-finding).

## Approve the checked spans

Only a human runs the commands in this section. Replace `your-name` with your reviewer name. Use span numbers from the current review output; editing a note can change them.

To approve one checked span, for example span 1:

```console
$ zettel note review retries-stop-on-all-errors --approve 1 --reviewer your-name
1 span(s) approved

```

To approve every remaining unreviewed agent span after checking each one:

```console
$ zettel note review retries-stop-on-all-errors --approve all --reviewer your-name
1 span(s) approved

```

The command reports the number of approved spans and records review metadata. Approval retains the agent provenance. It does not relabel agent text as human-authored text.

If a numbered approval fails, rerun the review listing. The number must identify an agent span in the current body. A human or citation span is not an agent span to approve.

## Verify the reusable content

```console
$ zettel note show retries-stop-on-all-errors
ab12-retries-stop-on-all-errors

  Title:        Retries stop on all errors
  Provenance:   agent:inference
  Tags:         retry

<!-- prov agent:inference reviewed=2026-01-01 reviewer=your-name -->
The client stops after any failed response.

<!-- /prov -->
<!-- prov agent:summary src=tests/retry.rs reviewed=2026-01-01 reviewer=your-name -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->

$ zettel note list --unreviewed

$ zettel read --provenance human,citation,reviewed
--- ab12-retries-stop-on-all-errors ---
title: Retries stop on all errors
provenance: agent:inference
created: 2026-01-01T00:00:00Z
tags: retry

[agent:inference reviewed=2026-01-01 reviewer=your-name]
The client stops after any failed response.


[agent:summary src=tests/retry.rs reviewed=2026-01-01 reviewer=your-name]
The retry tests send a 503 response and a 400 response.


$ zettel check
no problems found

```

The full note contains the review metadata. The unreviewed list is now empty; a partially approved note stays in that list while it has unreviewed agent content. The filtered read includes approved content and excludes unreviewed agent spans. `zettel check` reports structural problems; it does not judge the claims.

Review the Markdown diff and commit the approved note with the relevant work. A later session can read its evidence and review state from the repository.

## Correct a finding

When evidence overturns a finding or changes its scope, create a correction note linked to the old note. The steps below correct `retries-stop-on-all-errors`; use the old note's ID from your own store.

First, read the old note and its backlinks:

```console
$ zettel note show retries-stop-on-all-errors
ab12-retries-stop-on-all-errors

  Title:        Retries stop on all errors
  Provenance:   agent:inference
  Tags:         retry

<!-- prov agent:inference reviewed=2026-01-01 reviewer=your-name -->
The client stops after any failed response.

<!-- /prov -->
<!-- prov agent:summary src=tests/retry.rs reviewed=2026-01-01 reviewer=your-name -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->

$ zettel backlinks retries-stop-on-all-errors

```

Inspect each dependent note that the backlinks name. Identify each conclusion that relies on the old claim.

Create the correction note. The example uses synthetic test names. Replace the title and body with the real result. Include the test command and result, or a source citation. The example labels agent text. When a person writes the correction, replace `agent:summary` with `human:your-name` in this step and the next.

```console
$ zettel note create "Retry behavior differs by response" -t correction -p agent:summary -l retries-stop-on-all-errors -b 'Correction to the linked finding: the synthetic retry_503 test retries a 503 response; retry_400 stops after a 400 response. The earlier statement applied one behavior to both cases.'
cd34-retry-behavior-differs-by-response

```

Link the old note to the correction, and tag it:

```console
$ zettel note edit retries-stop-on-all-errors --add-link retry-behavior-differs-by-response --add-tag superseded

```

Keep the old body. Open the old note's file under `.zettel/` in an editor, and add a notice with its own provenance marker at the end. Each marker goes on its own line. The file then reads:

```markdown {filename=".zettel/ab12-retries-stop-on-all-errors.md"}
---
title: Retries stop on all errors
provenance: agent:inference
tags:
- retry
- superseded
links:
- retry-behavior-differs-by-response
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

<!-- prov agent:inference reviewed=2026-01-01 reviewer=your-name -->
The client stops after any failed response.

<!-- /prov -->
<!-- prov agent:summary src=tests/retry.rs reviewed=2026-01-01 reviewer=your-name -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->

<!-- prov agent:summary -->
Correction recorded in [[retry-behavior-differs-by-response]]. Read that note before reusing this finding.
<!-- /prov -->
```

The new span has `agent:summary` provenance regardless of the note's default. It stays unreviewed until a human approves it, so the old note returns to the unreviewed list, beside the correction:

```console
$ zettel note list --unreviewed
ab12-retries-stop-on-all-errors  agent:inference  [retry, superseded]  Retries stop on all errors
cd34-retry-behavior-differs-by-response  agent:summary  [correction]  Retry behavior differs by response

```

A `zettel note edit --body` or `--append` edit removes a review stamp from the frontmatter provenance, so earlier unmarked text also returns to the unreviewed list. A stamp on a body marker stays.

Zettel has no supersession state. The `superseded` tag is a convention. Reads, searches, and provenance filters do not treat it specially. A program that reads notes must define how it handles this tag.

Update each dependent conclusion to reflect the evidence. Then check both notes and their links:

```console
$ zettel context retry-behavior-differs-by-response --depth 1
--- cd34-retry-behavior-differs-by-response ---
title: Retry behavior differs by response
provenance: agent:summary
tags: correction
links: retries-stop-on-all-errors

Correction to the linked finding: the synthetic retry_503 test retries a 503 response; retry_400 stops after a 400 response. The earlier statement applied one behavior to both cases.

--- ab12-retries-stop-on-all-errors ---
title: Retries stop on all errors
provenance: agent:inference
tags: retry, superseded
links: retry-behavior-differs-by-response

<!-- prov agent:inference reviewed=2026-01-01 reviewer=your-name -->
The client stops after any failed response.

<!-- /prov -->
<!-- prov agent:summary src=tests/retry.rs reviewed=2026-01-01 reviewer=your-name -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->

<!-- prov agent:summary -->
Correction recorded in [[retry-behavior-differs-by-response]]. Read that note before reusing this finding.
<!-- /prov -->

$ zettel backlinks retries-stop-on-all-errors
cd34-retry-behavior-differs-by-response  Retry behavior differs by response

$ zettel check
no problems found

```

The context output includes both notes. The backlinks output for the old note includes the correction. `zettel check` reports structural problems. It does not judge claims.

Review the diff. Commit both notes together with any dependent note changes. Git keeps earlier versions.

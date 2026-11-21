```console
$ zettel init
initialized a zettel store at [CWD]

$ zettel note create "Retries stop on all errors" -t retry -p agent:inference -b 'The client stops after any failed response.'
ab12-retries-stop-on-all-errors

$ zettel note create "Retry behavior differs by response" -t correction -p agent:summary -l retries-stop-on-all-errors -b 'Correction to the linked finding.'
cd34-retry-behavior-differs-by-response

```

```markdown {filename=".zettel/ab12-retries-stop-on-all-errors.md"}
---
title: Retries stop on all errors
provenance: agent:inference
tags:
- retry
- superseded
links:
- cd34-retry-behavior-differs-by-response
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

The client stops after any failed response.

<!-- prov agent:summary -->
Correction recorded in [[cd34-retry-behavior-differs-by-response]]. Read that note before reusing this finding.
<!-- /prov -->
```


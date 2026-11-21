```console
$ zettel init
initialized a zettel store at [CWD]

$ zettel note create "Retries stop on all errors" -t retry -p agent:inference -b 'The client stops after any failed response.'
ab12-retries-stop-on-all-errors

```

```markdown {filename=".zettel/ab12-retries-stop-on-all-errors.md"}
---
title: Retries stop on all errors
provenance: agent:inference
tags:
- retry
links: []
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
---

The client stops after any failed response.

<!-- prov agent:summary src=tests/retry.rs -->
The retry tests send a 503 response and a 400 response.
<!-- /prov -->
```


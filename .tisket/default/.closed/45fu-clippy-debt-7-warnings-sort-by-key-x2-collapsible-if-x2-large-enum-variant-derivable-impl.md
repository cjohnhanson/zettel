---
title: 'clippy debt: 7 warnings (sort_by_key x2, collapsible if x2, large enum variant, derivable impl)'
status: done
priority: '4'
assignee: null
due_date: null
labels:
- lint
depends_on: []
created: 2026-08-12T21:15:24Z
updated: 2026-11-09T15:41:26Z
---

Pre-existing before the prose conversion. Zero-warning discipline applies across the ecosystem; these predate the extraction.

## Scratch Notes

Verified on main after #3: cargo clippy --all-targets reports zero warnings. The seven warnings were resolved during the provenance work (sort_by_key conversions landed with it; the large-enum/derivable-impl sites were removed with the Status enum).
> closed in triage 2026-11-09: status was already done; warnings cleared in 233014e and PR #3, close recorded in PR #4 (https://github.com/cjohnhanson/zettel/pull/4) but the file was never moved to .closed

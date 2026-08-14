---
title: "clippy debt: 7 warnings (sort_by_key x2, collapsible if x2, large enum variant, derivable impl)"
status: done
priority: 4
assignee:
labels: [lint]
depends_on: []
created: 2026-08-12T21:15:24Z
updated: 2026-08-14T19:51:46Z
---

Pre-existing before the prose conversion. Zero-warning discipline applies across the ecosystem; these predate the extraction.

## Scratch Notes

Verified on main after #3: cargo clippy --all-targets reports zero warnings. The seven warnings were resolved during the provenance work (sort_by_key conversions landed with it; the large-enum/derivable-impl sites were removed with the Status enum).

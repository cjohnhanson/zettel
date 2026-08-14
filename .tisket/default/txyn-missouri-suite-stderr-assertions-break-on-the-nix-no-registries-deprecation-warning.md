---
title: "missouri suite: stderr assertions break on the nix --no-registries deprecation warning"
status: todo
priority: 2
assignee:
labels: [tests]
depends_on: []
created: 2026-08-12T21:15:24Z
updated: 2026-08-12T21:15:24Z
---

The suite was unrunnable until the bin shim path fix (stale ../../../../.. from the monorepo extraction, now corrected). With the shim fixed, 6 assertions fail on one cause: nix prepends 'warning: --no-registries is deprecated; use --no-use-registries' to stderr, and the expected strings match only after that line. Fix direction: normalize or filter the nix wrapper warning in the comparison, or pin the sandbox invocation flags.

## Scratch Notes

Verified: the full suite passes with MISSOURI_SANDBOX=preinstalled (8 passed, 409 assertions). The root fix belongs in missouri: its nix sandbox invocation emits the deprecation warning into stderr that transitions assert on. Until missouri filters its own wrapper output, run this suite with MISSOURI_SANDBOX=preinstalled.

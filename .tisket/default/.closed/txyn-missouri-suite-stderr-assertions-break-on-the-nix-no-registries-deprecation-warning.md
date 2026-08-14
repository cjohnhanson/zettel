---
title: "missouri suite: stderr assertions break on the nix --no-registries deprecation warning"
status: done
priority: 2
assignee:
labels: [tests]
depends_on: []
created: 2026-08-12T21:15:24Z
updated: "2026-08-14T14:42:06Z"
---

The suite was unrunnable until the bin shim path fix (stale ../../../../.. from the monorepo extraction, now corrected). With the shim fixed, 6 assertions fail on one cause: nix prepends 'warning: --no-registries is deprecated; use --no-use-registries' to stderr, and the expected strings match only after that line. Fix direction: normalize or filter the nix wrapper warning in the comparison, or pin the sandbox invocation flags.

## Scratch Notes

Verified: the full suite passes with MISSOURI_SANDBOX=preinstalled (8 passed, 409 assertions). The root fix belongs in missouri: its nix sandbox invocation emits the deprecation warning into stderr that transitions assert on. Until missouri filters its own wrapper output, run this suite with MISSOURI_SANDBOX=preinstalled.
Resolved by dropping the `packages: [jq]` line from tests/missouri/.missouri/missouri.yml. A packages list makes missouri wrap every command in `nix shell`; that nix prepends "warning: '--no-registries' is deprecated" to stderr, and six assertions compare stderr exactly. jq now comes from PATH.

Before: 0 passed, 8 failed, 72 assertions reached, 1m20s wall / 10m CPU.
After:  8 passed, 0 failed, 409 assertions, 2.5s wall / 16.4s CPU.

The root fix still belongs in missouri: a suite should be able to declare nix packages without the wrapper leaking into stderr. This change works around it in zettel.

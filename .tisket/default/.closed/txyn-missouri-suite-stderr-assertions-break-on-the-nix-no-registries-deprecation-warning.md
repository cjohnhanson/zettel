---
title: "missouri suite: stderr assertions break on the nix --no-registries deprecation warning"
status: done
priority: 2
assignee:
labels: [tests]
depends_on: []
created: 2026-08-12T21:15:24Z
updated: "2026-08-14T14:59:01Z"
---

The suite was unrunnable until the bin shim path fix (stale ../../../../.. from the monorepo extraction, now corrected). With the shim fixed, 6 assertions fail on one cause: nix prepends 'warning: --no-registries is deprecated; use --no-use-registries' to stderr, and the expected strings match only after that line. Fix direction: normalize or filter the nix wrapper warning in the comparison, or pin the sandbox invocation flags.

## Scratch Notes

Verified: the full suite passes with MISSOURI_SANDBOX=preinstalled (8 passed, 409 assertions). The root fix belongs in missouri: its nix sandbox invocation emits the deprecation warning into stderr that transitions assert on. Until missouri filters its own wrapper output, run this suite with MISSOURI_SANDBOX=preinstalled.
Root cause found, and it is not in zettel. missouri's NixBackend::nix_prefix_args passes nix the deprecated '--no-registries' instead of '--no-use-registries'. nix 2.34.1 answers with 'warning: --no-registries is deprecated' on stderr, and that line merges into the stderr of every command under test. Any suite that declares packages and asserts on stderr breaks.

zettel's config was never wrong. packages: [jq] stays.

A non-hermetic workaround (dropping packages, taking jq from PATH) was merged as #1 and is now reverted. Hermeticity is not a tradeable cost; a suite that takes its tools from the ambient PATH no longer tests what it claims to.

The fix belongs in missouri and Cody has it in flight in another session. This suite stays red against the released missouri until that lands. No further change is needed here once it does.
FIXED in missouri (codelikecody retire-clc-config): executor now passes --no-use-registries instead of the deprecated --no-registries, so no warning pollutes asserted stderr. The zettel suite passes 8/8 in full nix mode with the fixed binary. The system missouri stays stale until the pin bump + hms.

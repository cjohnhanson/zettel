---
title: 'composable stores: multi-store composition for zettel and tisket'
status: in_progress
priority: '2'
assignee: null
due_date: null
labels:
- feature
depends_on: []
created: '2026-08-14T20:27:44Z'
updated: '2026-08-14T20:29:35Z'
---

## End goal

Multiple document stores — repo-level, user-level, remote — composed as a directed graph of linkability, shared by zettel and tisket. A user-level store links into repo stores; repo stores never link back (they cannot: the target does not exist for other clones). Mutual cycles between shareable stores are sound.

Design + review + phased plan: ~/.artifacts/composable-stores/ (http://localhost:8080/composable-stores/)

## Settled decisions

1. Ref transitivity: direct-only refs, transitive traversal.
2. Machine-local registry: in v1, keyed by declared source.
3. Epics: a `children:` field (containment is not blocking).
4. Remote scope: paths + git + blob, all in v1, sequenced across phases.

## Design revisions from the adversarial review (17)

Identity/resolution: store identity is the declared source, not a self-declared name (R1); resolution takes the referring document's home store, and an alias resolves through THAT store's table (R2); one stores.yml per repo, both tools read it (R3).

Output: non-local ids print store-qualified (R4); printed ids must round-trip (R5); transitive results get a path form project/shared-kb:id (R6); store attribution is the prompt-injection defense (R7).

Trust: shareability is structural — a shared config's path dep must resolve inside its own repo, siblings must use git URLs (R8); provenance never coalesces across stores, a dep's human/reviewed never merges into vantage counts or matches unqualified filters (R9); symlink and root-escape guards at the open path (R10); dep stores are read-only through the closure (R11).

Remotes: bare clones with rev-addressed reads, never a shared working tree (R12); the registry cannot make check lie — dual-resolution warning (R13); partial closures are labeled, never silently wrong (R14).

Foundations: a snapshot load model, which also fixes zettel's existing quadratic stats/backlinks (R15); the layer splits — mdstore keeps pure parts, resolution behind a trait, git in an opt-in module (R16); one grammar discrimination rule (alias only when the pre-colon head is declared) plus a format version guard (R17).

## Phases

1. Foundations + local paths (mdstore StoreRef/config/graph/Snapshot/guards; zettel adopts) — delivers user-store -> repo-store linking end to end.
2. Git sources (bare cache, rev-addressed reads, store list/sync, partial-closure annotations).
3. Registry + shareability enforcement.
4. tisket adoption: stores.yml, children:, rollup, and a new `tisket check` command.
5. Blob sources (adapter + credential surface; no model change).

## Success criteria

- A user store links into repo stores; backlinks/context/stats are correct across the closure and label partial results.
- A repo store cannot resolve a dep that other clones cannot reach.
- A dep store's provenance claims never merge into the vantage store's trust counts.
- Existing single-store repos work untouched.

## Scratch Notes

Review artifact: ~/.artifacts/composable-stores/. Raw verified concerns: scratchpad/stores-review.json (34 kept, 9 high). Workflow run wf_47aded59-44e — verify:architecture, verify:migration, and synthesize-plan died on a Fable limit; those findings were hand-verified instead (tisket selector/slug are already mdstore re-exports, not forks; zettel config has no version field; tisket depends_on already supports colon-bearing entries per issue.rs:297).
## Phase 1 QA plan (written before test code)

### mdstore: StoreRef grammar
1. Bare id parses with no alias: "a3f2" -> {alias: None, id: a3f2}.
2. "project:a3f2" -> alias project, id a3f2.
3. Path form "project/shared-kb:b7c1" -> alias path [project, shared-kb], id b7c1 (read-only form).
4. Discrimination rule: parse takes the declared-alias set. "10.1145/12345" with no alias named "10.1145" stays an opaque bare id. "x: y" (tisket legacy) stays opaque.
5. An id containing a colon after a declared alias head: "project:a:b" -> alias project, id "a:b" (split on FIRST colon only).
6. Display round-trips every form; empty alias or empty id is an error.
7. Path form with an undeclared head is opaque, not an error.

### mdstore: StoresConfig / stores.yml
8. Missing stores.yml -> empty config, no error (every existing repo).
9. Unknown keys preserved (do not clobber a user's file).
10. A dep with neither path nor git nor blob -> error naming the alias.
11. Duplicate alias in one file -> error (serde_yml may take last; assert explicitly).
12. format key absent = format 1; format > supported = hard error naming the needed upgrade.

### mdstore: identity and dedup
13. Two aliases for the same git URL dedup to one closure member.
14. A path dep whose origin remote equals a git dep's URL dedups to one member; the nearer (path) source wins for content.
15. A path dep with no git origin gets a path-based identity; two aliases to the same canonical path dedup.
16. Distinct sources never dedup.
17. Same declared name, different identity -> a finding, not a merge.

### mdstore: graph traversal
18. A -> B -> C closure from A yields A, B, C once each.
19. Mutual cycle A <-> B terminates and yields both once.
20. Self-reference terminates.
21. A diamond (A->B, A->C, B->D, C->D) yields D once.
22. Traversal order is deterministic (declaration order, breadth-first).

### mdstore: guards
23. A store dir config value that is absolute -> error.
24. A store dir that escapes the root via .. after canonicalization -> error.
25. A symlinked .md file is skipped, and the skip is reported (not silently dropped).
26. A symlinked directory inside the store dir is skipped.
27. A non-regular file (fifo) is skipped.
28. A .md file whose real path is inside the store is read normally (no false positives).

### mdstore: Snapshot
29. Each store's files are read exactly once for a full graph pass (assert via a counting loader).
30. Forward edges recorded per (store, doc); reverse index derives backlinks without rescanning.
31. A doc with no edges appears in neither index but is present in the doc map.
32. An unreadable member is reported in availability, and the snapshot still builds from the rest.
33. Bare refs resolve within the containing store only: two stores each holding "provenance-model" resolve to their own.
34. An alias ref resolves through the containing store's table, NOT the vantage's (confused-deputy case: dep declares project->X, vantage declares project->Y; a dep doc's project:a3f2 resolves to X).
35. A ref to an undeclared alias resolves to nothing and is reported as a finding against the containing store.

### zettel: cross-store behavior
36. A user store declaring project by path: note list from user vantage shows local notes bare and project notes as project:<id>.
37. backlinks on a project note from user vantage includes the user's annotation note; from project vantage it does not (project does not declare the user store).
38. context crosses the boundary and labels foreign notes.
39. A [[project:a3f2]] body ref and a links: [project:a3f2] frontmatter entry both resolve.
40. citation:project:a3f2 is a cross-store graph edge; citation:kleppmann (undeclared head) stays an external key with no finding.
41. Declaring a new alias that shadows an existing citation qualifier -> check finding (meaning change surfaced once).
42. orphans from the user vantage: a project note whose only inbound link is a user note is NOT an orphan; from project vantage it IS.
43. stats counts each store's notes once in a diamond.
44. Provenance never coalesces: stats from user vantage reports dep spans store-qualified; --provenance human matches only vantage-local human spans.
45. Mutating commands reject alias-qualified ids: note edit project:a3f2 errors naming the store to run from.
46. check findings: undeclared alias, out-of-repo path in a shared config, skipped symlink, shadowed qualifier.
47. Every printed id round-trips: an id copied from list/context/backlinks output resolves through the same command's input grammar.
48. An existing single-store repo with no stores.yml behaves exactly as today (regression).

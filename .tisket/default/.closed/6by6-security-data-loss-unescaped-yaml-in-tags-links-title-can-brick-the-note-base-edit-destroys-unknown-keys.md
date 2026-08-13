---
title: "SECURITY/DATA-LOSS: unescaped YAML in tags/links/title can brick the note base; edit destroys unknown keys"
status: done
priority: 1
assignee:
labels: [bug, data-loss]
depends_on: []
created: 2026-08-13T02:06:36Z
updated: "2026-08-13T17:38:22Z"
---

src/note.rs:113 emits tags: [{joined}] and :107 writes title: "{}" escaping only the quote, not the backslash — a tag/link/title with YAML metacharacters or a trailing backslash produces a file that no longer parses, and one bad file can break repo-wide commands. src/note.rs:105-142 rebuilds frontmatter from six known fields, so note edit silently drops any unknown frontmatter key. Fix: serialize via serde_yml (the mdstore path) instead of hand-rolled format!, and carry unknown keys with a flatten catch-all. Also: zettel has ZERO unit tests over 1688 lines.

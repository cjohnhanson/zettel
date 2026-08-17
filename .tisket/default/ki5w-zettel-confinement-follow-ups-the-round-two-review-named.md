---
title: 'zettel: confinement follow-ups the round-two review named'
status: todo
priority: null
assignee: null
due_date: null
labels:
- review-followup
depends_on: []
created: 2026-08-17T02:28:41Z
updated: 2026-08-17T02:28:41Z
---

The round-two fresh-eyes review returned LAND on the confinement branch
and named three follow-ups. Each is recorded so the reasoning survives.

1. Hold the handle lazily, in a OnceCell rather than eagerly in
   Repo::open. Two things follow.

   The read-only corner closes. A store on a read-only mount whose note
   directory is absent now fails every command with a bare Permission
   denied, where the merge base answered an empty list. list, check and
   stats never need the directory to exist.

   A read command stops performing a mkdir. Given `zettel_dir:
   link/notes` with `link -> ../outside`, document_dir's canonicalize
   check is skipped because the joined path does not exist yet, so
   create_dir_all follows the link. `zettel note list` on such a store
   creates outside/notes where the merge base created nothing. The
   branch already improves on the merge base here, which wrote the note
   outside on every command and never self-corrected; this makes the
   trigger narrower still.

   The anti-swap property survives lazily: the handle is opened once
   and held, just on first use.

2. Assert the id guard's in-store half. is_plain_stem refuses an empty
   id and a leading dot, and the handle cannot, because both names are
   inside the store. With the guard removed, `note delete ''` removes
   .zettel/.md and `note delete .hidden` removes .zettel/.hidden.md,
   both exit 0. The existing test passes with the guard deleted, so its
   comment overstates what it asserts. Two ids and a planted dotfile in
   the fixture fix it.

3. Drop the dead `if !path.exists()` in find_note. Removing it leaves
   every test green. It is the last unconfined path probe on a
   caller-derived path in the note surface, and it follows links, which
   is the predicate this whole branch argues against.

Two smaller notes: a broken symlink as the whole zettel_dir reports a
bare `File exists (os error 17)`, and the delete of a directory named
x.md went from a leaked errno to a clean not-found with nothing pinning
it.

The underlying document_dir weakness belongs to mdstore, not here: it
should refuse a path whose longest existing prefix resolves outside the
store root.

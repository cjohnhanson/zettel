# Contributing

## Use of AI

Be able to explain the change in your own words, and write your
own comments on the pull request. A pull request the author cannot
explain is closed.

## Setup

```sh
git clone https://github.com/cjohnhanson/zettel
cd zettel
cargo build
cargo test --workspace --all-features
```

Some tests need other programs on `PATH`. Install `node` 18 or later
from your package manager. Install the other two from source:

```sh
cargo install --locked --git https://github.com/cjohnhanson/missouri
cargo install --locked --git https://github.com/cjohnhanson/gaff
```

`node` drives the npm wrapper tests, `missouri` runs the state suite,
and `gaff` runs the merge-gate tests.

## Before a large change

Open a GitHub issue first. A small fix needs none.

The maintainer tracks work in markdown files under `.tisket/`. Read
them with `cat`, or with [tisket](https://github.com/cjohnhanson/tisket)
(`tisket issue list`, `tisket issue show <id>`). Do not edit them in a
pull request.

## What CI checks

Four workflows check a change, and `release.yml` publishes a tag.
`verify` runs on every push. `gate` runs on a pull request, and `main`
requires it. `review-gate` runs on a pull request too. It hands the
sign-off note to `gaff reviews check` under the policy on `main`, so a
branch cannot weaken the check that judges it. `docs` runs on a pull
request and on a push to `main`, and runs the checks described under
"Docs tests".

`verify` and `gate` run the commands the local git hooks run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd tests/missouri && missouri run
```

After those, `gate` reads a review note from the head commit of the
branch. It reads the branch head, not the merge commit GitHub builds for
CI, because no reviewer saw that merge commit.

The review note is the maintainer's part. `.gaff/gaff.yml` names the
reviews a change must pass (`gaff reviews` prints them): `review-tests`,
`review-docs`, `review-code`, `review-deps`, and `review-usability`. For
each one, an agent that did not write the change reads it against the
criteria in `.agents/skills/<name>/SKILL.md`. It returns one line, which
names the commit it read:

```
signoff[review-tests] PASS 4f1c2ab removed a guard, a_failed_signoff went red
```

Every line goes in one git note on that commit:

```sh
git notes --ref=reviews add -m '<the lines>' <sha>
```

`.agents/skills/signoff-driver/SKILL.md` has the procedure. As a
contributor from outside, open the pull request and stop there. The
maintainer runs the reviews on its head commit and writes the note.

Both checks refuse a note with any of these faults:

- no line for a declared review
- a `FAIL` verdict
- a line that names another commit, or a sha under seven characters
- two lines for one review
- fewer than three words of evidence

Prose around the lines is ignored, so a note can also carry a narrative.

## Docs tests

`tests/docs.rs` runs every `console` block in README.md and docs/*.md
against the built binary with trycmd. Each page runs in a scratch
directory with its own `HOME`, a cleared environment, and a fixed git
identity. The notes a page creates take fixed ID prefixes in order:
`ab12`, `cd34`, `ef56`, and so on. Every `created` and `updated` value
in the page's notes becomes `2026-01-01T00:00:00Z`, and every
`reviewed=` date becomes `2026-01-01`. A fence
such as `` ```markdown {filename=".zettel/ab12-a.md"} `` is a file the
reader writes; the test writes it before the commands that follow.
Fixtures for a page live in `tests/docs/<page>/`.

The test also compares the part of docs/cli-reference.md between its
markers with the `--help` output of every command. Change help text in
the doc comments in `src/cli.rs`, never in the page. To write real
output into the pages and the reference, run one of these, then read
the diff:

```sh
TRYCMD=overwrite cargo test --test docs
SNAPSHOTS=overwrite cargo test --test docs
```

In README.md and docs/*.md, put every zettel command in a `console`
block, except under Install. A `sh`, `text`, or other tagged block never
runs. Give every fence a language: trycmd runs a fence with none.

The `docs` workflow calls `.github/workflows/family-checks.yml` in the
[cjohnhanson/diataxis](https://github.com/cjohnhanson/diataxis)
repository. The same docs checks run for zettel and its sibling tools:
tundish, gaff, tisket, almanac, diataxis, and missouri. It reads
README.md and docs/, and fails when:

- `tests/docs.rs` is missing, or does not name trycmd, README.md, and
  docs;
- a command of any of those tools sits outside a `console` block, except
  under Install;
- a fence has no language, or is a `trycmd` fence;
- the README does not open with `# zettel`, has no paragraph before its
  first section, has no `## Install` section, or does not end with
  `## License`;
- a link to a file or heading in the repository is broken (a push to
  `main` checks web links too, except the hosts in `.lycheeignore`,
  which refuse automated requests);
- typos finds a misspelling.

The scripts and their rules are in the diataxis repository under
`scripts/family/`; each one prints its rules with `--help`.

## Running the gates locally

`.gaff/gaff.yml` declares the gates once. `gaff init --github` writes
the `verify` and `gate` workflows from it, and `gaff init --git` writes
the hooks. To run the gates against `HEAD` without a commit:

```sh
cargo install --locked --git https://github.com/cjohnhanson/gaff
gaff ci
```

The pre-push hook refuses a push that carries no review note. For a
one-off contribution, leave the hooks uninstalled and let CI run the
gates on the pull request.

## Pull requests

Branch from `main` and open the pull request from a fork. Keep a change
and its tests in one pull request. Write the commit message in the
imperative present: what the change does, and why, where the diff does
not show it.

## Releases

Before a tag, run `sh scripts/prepublish.sh` from the repository root.
It needs `cargo-hack`, `cargo-audit`, `python3`, `curl`, and `rustup`
with the toolchain `rust-toolchain.toml` pins. It checks every feature
combination. It runs the tests, clippy, and fmt under the pinned
toolchain, and audits the lock. It runs a publish dry run from a clean
checkout and confirms the version is not on crates.io. The first
publish of a name to crates.io is a manual `cargo publish` with a
token, because trusted publishing cannot create a crate.

PyPI needs its own step before the first tag, and only a person can do
it. Trusted publishing does create a new project, but only through a
pending publisher, and a pending publisher exists only once someone
registers it. Register one at
<https://pypi.org/manage/account/publishing/> with project `zttl`,
owner `cjohnhanson`, repository `zettel`, workflow `release.yml`, and
no environment. Without it the tag publishes the crate and then fails
at `uv publish`, and crates.io never releases the version it took.

After both,
bump the version in `Cargo.toml`, commit, tag `v<version>`, and push
the tag. `.github/workflows/release.yml` builds and publishes from
there. A `workflow_dispatch` run of that workflow rehearses every
build and publishes nothing.

## What not to commit

`.gitignore` covers `target/`, editor and agent state, and
`.cargo/config.toml`. Two things it cannot catch. An absolute path that
names a home directory exposes an account name and breaks every other
clone. A `path = "..."` dependency override that points outside the
repository does the same; put it in `.cargo/config.toml`.

## Questions

Open a GitHub issue.

## Security

Do not open a public issue for a vulnerability. See
[SECURITY.md](SECURITY.md).

## License

Contributions are licensed under MIT, the same as the project.

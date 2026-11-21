//! The docs are tests.
//!
//! `console_examples` runs every `console` block in README.md and
//! docs/*.md against the built binary with trycmd, and compares the
//! printed output with the block. Each page runs top to bottom in its own
//! scratch directory, with `HOME` and the XDG directories in that
//! directory, so no page reads the machine's store registry, sync cache,
//! or git config. zettel reads its user config from the passwd home
//! directory, not `$HOME`, so the `zettel` wrapper also passes the hidden
//! `--user-config` flag with a path under the page's `HOME`.
//!
//! Every command a page runs starts through a small `/bin/sh` wrapper that
//! clears the environment (`env -i`) and sets only the variables below.
//! Without it an inherited variable changes the output: `GIT_DIR` from a
//! git hook sends a page's `git` commands to another repository, and
//! `MDSTORE_REGISTRY` or `XDG_CONFIG_HOME` points zettel at the machine's
//! registry. Git gets a fixed identity and a fixed clock, and
//! `GIT_EDITOR=true`, so a git command that opens an editor completes
//! instead of waiting for a terminal.
//!
//! A note ID starts with a random four-character prefix. The `zettel`
//! wrapper renames each file that `note create` writes so that its prefix
//! comes from `PREFIXES` in order: the first note a page creates is
//! `ab12-...`, the second `cd34-...`. It rewrites the ID that `note
//! create` prints to match. The rest of the ID, the slug, is the binary's
//! own.
//!
//! zettel stamps a note's `created` and `updated` frontmatter with the
//! current time, and an approval's `reviewed=` attribute with the current
//! date. The binary has no clock flag, so after every `zettel` command the
//! wrapper rewrites those values in the page's note files to
//! `2026-01-01T00:00:00Z` and `2026-01-01`, the date git gets. Every
//! command that prints a timestamp reads it from a note file, so the
//! output shows the fixed value too. Only the timestamp values change; a
//! line that holds anything else is left alone.
//!
//! A fence with `{filename="<path>"}` in its info string is a file the
//! reader writes. Its body is written to that path in the page's working
//! directory when the page reaches it, so the commands after it see the
//! file.
//!
//! A page can have fixtures in tests/docs/<page>/: `setup.md` holds
//! `console` blocks that run before the page, for examples that assume
//! earlier state; `work/` is copied into the working directory and
//! `home/` into `HOME`.
//!
//! `cli_reference_matches_help` compares the part of docs/cli-reference.md
//! between its markers with the `--help` output of every visible command.
//!
//! Write the real output into the pages, then read the diff:
//! `TRYCMD=overwrite cargo test --test docs` for the examples,
//! `SNAPSHOTS=overwrite cargo test --test docs` for the CLI reference.

use std::path::{Path, PathBuf};
use std::process::Command;

use clap::CommandFactory as _;

const BIN: &str = env!("CARGO_BIN_EXE_zettel");
/// clap wraps help to `COLUMNS` when no terminal is attached, so the
/// width is fixed for every run.
const COLUMNS: &str = "100";
/// The prefixes the `zettel` wrapper gives created notes, in order.
const PREFIXES: &[&str] = &[
    "ab12", "cd34", "ef56", "gh78", "jk90", "mn23", "pq45", "rs67", "tu89", "vw01", "xy23", "za45",
    "bc67", "de89", "fg01", "hj23", "kl45", "mp67", "qr89", "st01",
];
/// The value every `created:` and `updated:` timestamp takes.
const TIMESTAMP: &str = "2026-01-01T00:00:00Z";
/// The value every `reviewed=` date takes.
const DATE: &str = "2026-01-01";
const BEGIN: &str = "<!-- Begin generated CLI help: cargo test --test docs -->";
const END: &str = "<!-- End generated CLI help -->";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// README.md and every page in docs/, in name order.
fn pages() -> Vec<PathBuf> {
    let mut pages: Vec<PathBuf> = std::fs::read_dir(root().join("docs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    pages.insert(0, root().join("README.md"));
    pages
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn console_examples() {
    let guard = tempfile::tempdir().unwrap();
    // Canonical, so a tool that prints a resolved path prints the same
    // path trycmd writes as `[CWD]`.
    let scratch = guard.path().canonicalize().unwrap();
    let mut failed = Vec::new();
    for page in pages() {
        if std::panic::catch_unwind(|| run_page(&scratch, &page)).is_err() {
            failed.push(page.strip_prefix(root()).unwrap().display().to_string());
        }
    }
    assert!(
        failed.is_empty(),
        "examples differ from the binary in: {failed:?}"
    );
}

/// Run one page's `console` blocks in `scratch/<stem>/`.
fn run_page(scratch: &Path, page: &Path) {
    let stem = page.file_stem().unwrap().to_string_lossy().into_owned();
    let fixtures = root().join("tests/docs").join(&stem);
    let dir = scratch.join(&stem);
    // trycmd runs `page.md` in `page.in/`.
    let work = dir.join("page.in");
    let home = dir.join("home");
    let bin = dir.join("bin");
    for (from, to) in [
        (fixtures.join("work"), &work),
        (fixtures.join("home"), &home),
    ] {
        if from.is_dir() {
            copy_dir(&from, to);
        } else {
            std::fs::create_dir_all(to).unwrap();
        }
    }
    std::fs::create_dir_all(&bin).unwrap();
    let setup = std::fs::read_to_string(fixtures.join("setup.md")).unwrap_or_default();
    let body = std::fs::read_to_string(page).unwrap();
    let text = format!("{setup}{body}");

    // trycmd skips, and does not fail, a command it cannot resolve, and it
    // resolves an unregistered crate binary to the binary itself. So every
    // command a page runs is registered here, through a wrapper, from the
    // blocks trycmd runs, or the page fails.
    let mut registered = Vec::new();
    for name in commands(page, &text) {
        let wrapper = if name == "zettel" {
            write_zettel_wrapper(&bin)
        } else {
            let found = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|dir| dir.join(&name))
                .find(|p| p.is_file());
            let path = found.unwrap_or_else(|| {
                panic!("{} runs `{name}`, which is not on PATH", page.display())
            });
            write_wrapper(&bin, &name, &path)
        };
        registered.push((name, wrapper));
    }

    let case = dir.join("page.md");
    let overwrite = std::env::var_os("TRYCMD").is_some_and(|m| m == "overwrite");
    let mut written = String::new();
    for (index, segment) in segments(page, &text).into_iter().enumerate() {
        if segment.text.lines().any(|l| l.starts_with("$ ")) {
            std::fs::write(&case, &segment.text).unwrap();
            let cases = trycmd::TestCases::new();
            for (name, wrapper) in &registered {
                cases.register_bin(name.clone(), wrapper);
            }
            let home_var = |sub: &str| home.join(sub).display().to_string();
            cases
                .env("HOME", home.display().to_string())
                .env("XDG_CONFIG_HOME", home_var(".config"))
                .env("XDG_STATE_HOME", home_var(".local/state"))
                .env("XDG_DATA_HOME", home_var(".local/share"))
                .env("XDG_CACHE_HOME", home_var(".cache"))
                .case(&case)
                .run();
            if overwrite {
                written.push_str(&std::fs::read_to_string(&case).unwrap());
            } else {
                written.push_str(&segment.text);
            }
        } else {
            written.push_str(&segment.text);
        }
        if let Some((name, content)) = segment.file {
            assert!(
                !name.starts_with('/') && !name.split('/').any(|p| p == ".."),
                "{}: segment {index} writes {name}, outside the working directory",
                page.display()
            );
            let target = work.join(&name);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, content).unwrap();
        }
    }

    // TRYCMD=overwrite rewrote the scratch copies. Carry the page's part
    // back to the page; the setup part stays in tests/docs.
    if overwrite {
        let body = written.strip_prefix(&setup).unwrap_or_else(|| {
            panic!(
                "the setup for {} changed; run it again after updating setup.md",
                page.display()
            )
        });
        std::fs::write(page, body).unwrap();
    }
}

/// A run of a page that trycmd runs as one case, and the file the reader
/// writes at its end, if any.
struct Segment {
    text: String,
    file: Option<(String, String)>,
}

/// The value of `filename="..."` in a fence's info string.
fn filename(info: &str) -> Option<String> {
    let start = info.find("filename=\"")? + "filename=\"".len();
    let end = info[start..].find('"')?;
    Some(info[start..start + end].to_string())
}

/// Split a page after each fence that names a file, so the file can be
/// written before the commands that follow it run. Fences are read by the
/// same rules as `run_lines`.
fn segments(page: &Path, text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    // The open fence: its backticks, the file it names, and its body.
    let mut fence: Option<(String, Option<String>, String)> = None;
    for line in text.split_inclusive('\n') {
        current.push_str(line);
        let bare = line.trim_end_matches(['\n', '\r']);
        if let Some((marker, file, body)) = fence.as_mut() {
            if bare.starts_with(marker.as_str()) {
                let file = file.take();
                let content = std::mem::take(body);
                fence = None;
                if let Some(name) = file {
                    segments.push(Segment {
                        text: std::mem::take(&mut current),
                        file: Some((name, content)),
                    });
                }
            } else {
                body.push_str(line);
            }
            continue;
        }
        let ticks = bare.len() - bare.trim_start_matches('`').len();
        if ticks < 3 {
            continue;
        }
        let info = &bare[ticks..];
        let file = filename(info);
        assert!(
            file.is_none() || !info.trim_start().starts_with('{'),
            "{}: a file fence needs a language before {{filename=...}}",
            page.display()
        );
        fence = Some((bare[..ticks].to_string(), file, String::new()));
    }
    segments.push(Segment {
        text: current,
        file: None,
    });
    segments
}

/// The lines of every block trycmd runs, read by trycmd 1.2's own rules
/// (`parse_trycmd` in its schema.rs): a fence with no info string, or
/// one whose language is `console` or `trycmd`, runs; the block ends at
/// the first line that starts with the opening backticks.
///
/// Panics on a `console,ignore` or `trycmd,ignore` block: trycmd skips
/// it, so its commands would never be checked.
fn run_lines<'a>(page: &Path, text: &'a str) -> Vec<&'a str> {
    let mut lines = Vec::new();
    // The open fence: its backticks, and whether trycmd runs its body.
    let mut fence: Option<(&str, bool)> = None;
    for line in text.lines() {
        if let Some((marker, runs)) = fence {
            if line.starts_with(marker) {
                fence = None;
            } else if runs {
                lines.push(line);
            }
            continue;
        }
        let ticks = line.len() - line.trim_start_matches('`').len();
        if ticks < 3 {
            continue;
        }
        let info = line[ticks..].trim();
        let mut parts = info.split(',');
        let lang = parts.next().unwrap_or_default();
        let runs = info.is_empty() || lang == "console" || lang == "trycmd";
        assert!(
            !(runs && parts.any(|p| p == "ignore")),
            "{}: `{info}` hides its commands from the docs test",
            page.display()
        );
        fence = Some((&line[..ticks], runs));
    }
    lines
}

/// The command word of every `$ ` line in the blocks trycmd runs.
///
/// Panics on an output line holding `\`: trycmd reads `\` in real output
/// as `/`, so such a line can never match the binary.
fn commands(page: &Path, text: &str) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for line in run_lines(page, text) {
        if let Some(cmd) = line.strip_prefix("$ ") {
            if let Some(word) = cmd.split_whitespace().find(|w| !w.contains('=')) {
                names.insert(word.to_string());
            }
        } else if !line.starts_with("> ") {
            // `> ` continues a command; every other line is output.
            assert!(
                !line.contains('\\'),
                "{}: trycmd cannot match output holding a backslash; choose an example without one: {line}",
                page.display()
            );
        }
    }
    names
}

/// The variables every page command sees, and nothing else. `$HOME` and
/// the XDG directories are the page's, set by the test on the wrapper.
fn clean_env() -> String {
    format!(
        "env -i HOME=\"$HOME\" XDG_CONFIG_HOME=\"$XDG_CONFIG_HOME\" \
         XDG_STATE_HOME=\"$XDG_STATE_HOME\" XDG_DATA_HOME=\"$XDG_DATA_HOME\" \
         XDG_CACHE_HOME=\"$XDG_CACHE_HOME\" PATH=\"$PATH\" NO_COLOR=1 TERM=dumb \
         COLUMNS={COLUMNS} GIT_CONFIG_NOSYSTEM=1 GIT_EDITOR=true \
         GIT_AUTHOR_NAME=Ada GIT_AUTHOR_EMAIL=ada@example.com \
         GIT_COMMITTER_NAME=Ada GIT_COMMITTER_EMAIL=ada@example.com \
         GIT_AUTHOR_DATE={TIMESTAMP} GIT_COMMITTER_DATE={TIMESTAMP}"
    )
}

fn quoted(path: &Path) -> String {
    // The wrappers quote paths in single quotes.
    assert!(
        !path.to_string_lossy().contains('\''),
        "cannot wrap a path holding a single quote: {}",
        path.display()
    );
    format!("'{}'", path.display())
}

fn write_script(path: &Path, script: &str) -> PathBuf {
    std::fs::write(path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path.to_path_buf()
}

/// `name` in the examples: `target` with the clean environment.
fn write_wrapper(bin: &Path, name: &str, target: &Path) -> PathBuf {
    write_script(
        &bin.join(name),
        &format!(
            "#!/bin/sh\nexec {} {} \"$@\"\n",
            clean_env(),
            quoted(target)
        ),
    )
}

/// `zettel` in the examples: the built binary with the clean environment,
/// and `--user-config` under the page's `HOME`, because zettel reads its
/// user config from the passwd home directory. The flag is global and its
/// last occurrence wins, so a page can still pass its own.
///
/// After every command, the timestamps in the note files under the
/// working directory take `TIMESTAMP` and `DATE`. After `note create`,
/// each new note file takes the next prefix in `PREFIXES`, and the ID the
/// command printed changes with it; the count lives in `$HOME`, so it
/// runs across the page's blocks.
fn write_zettel_wrapper(bin: &Path) -> PathBuf {
    let mut pick = String::new();
    for (n, prefix) in PREFIXES.iter().enumerate() {
        pick.push_str(&format!("    {n}) p={prefix} ;;\n"));
    }
    write_script(
        &bin.join("zettel"),
        &format!(
            "#!/bin/sh\n\
             run() {{ {env} {bin} --user-config \"$HOME/.config/zettel/config.yml\" \"$@\"; }}\n\
             notes() {{ find . -path '*/.zettel/*' -name '*.md' 2>/dev/null | sort; }}\n\
             fix_times() {{\n\
             \x20 for f in $(notes); do\n\
             \x20   sed -E \
             -e 's/^(created|updated): [0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}T[0-9]{{2}}:[0-9]{{2}}:[0-9]{{2}}Z$/\\1: {TIMESTAMP}/' \
             -e 's/reviewed=[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}/reviewed={DATE}/g' \
             \"$f\" > \"$f.docs-test\" && mv \"$f.docs-test\" \"$f\" || exit 1\n\
             \x20 done\n\
             }}\n\
             case \" $* \" in\n\
             *' note create '*) ;;\n\
             *) run \"$@\"; code=$?; fix_times; exit $code ;;\n\
             esac\n\
             before=$(notes)\n\
             out=$(run \"$@\")\n\
             code=$?\n\
             for f in $(notes); do\n\
             \x20 printf '%s\\n' \"$before\" | grep -qxF \"$f\" && continue\n\
             \x20 n=$(cat \"$HOME/.docs-test-ids\" 2>/dev/null || echo 0)\n\
             \x20 echo $((n + 1)) > \"$HOME/.docs-test-ids\"\n\
             \x20 case $n in\n{pick}\
             \x20   *) echo 'tests/docs.rs: out of PREFIXES' >&2; exit 1 ;;\n\
             \x20 esac\n\
             \x20 base=${{f##*/}}\n\
             \x20 old=${{base%.md}}\n\
             \x20 new=$p-${{old#????-}}\n\
             \x20 mv \"$f\" \"${{f%/*}}/$new.md\" || exit 1\n\
             \x20 out=$(printf '%s\\n' \"$out\" | sed \"s/^$old\\$/$new/\")\n\
             done\n\
             fix_times\n\
             [ -n \"$out\" ] && printf '%s\\n' \"$out\"\n\
             exit $code\n",
            env = clean_env(),
            bin = quoted(Path::new(BIN)),
        ),
    )
}

/// Every visible command path, depth first: `[]`, `["init"]`, ...
fn command_paths(cmd: &clap::Command, prefix: &[String], out: &mut Vec<Vec<String>>) {
    out.push(prefix.to_vec());
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() || sub.get_name() == "help" {
            continue;
        }
        let mut path = prefix.to_vec();
        path.push(sub.get_name().to_string());
        command_paths(sub, &path, out);
    }
}

fn help(path: &[String]) -> String {
    let out = Command::new(BIN)
        .args(path)
        .arg("--help")
        .env_clear()
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("COLUMNS", COLUMNS)
        .output()
        .unwrap();
    assert!(out.status.success(), "{path:?} --help failed");
    String::from_utf8(out.stdout).unwrap()
}

/// The generated part of the CLI reference: one section per visible
/// command, with its `--help` output.
fn generated_reference() -> String {
    let mut paths = Vec::new();
    command_paths(&zettel::cli::Args::command(), &[], &mut paths);
    let mut text = String::new();
    for path in paths {
        let name = std::iter::once("zettel".to_string())
            .chain(path.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        text.push_str(&format!("\n## {name}\n\n```text\n{}```\n", help(&path)));
    }
    text
}

/// docs/cli-reference.md holds, between its markers, the `--help` output
/// of every visible command and nothing else.
#[test]
fn cli_reference_matches_help() {
    let path = root().join("docs/cli-reference.md");
    let page = std::fs::read_to_string(&path).unwrap();
    let begin = page
        .find(BEGIN)
        .expect("docs/cli-reference.md lacks the begin marker")
        + BEGIN.len();
    let end = page
        .find(END)
        .expect("docs/cli-reference.md lacks the end marker");
    let generated = format!(
        "{}\n{}\n{}",
        &page[..begin],
        generated_reference(),
        &page[end..]
    );
    snapbox::Assert::new()
        .action_env("SNAPSHOTS")
        .eq(generated, snapbox::Data::read_from(&path, None).raw());
}

/// The harness itself: a page whose output differs from the binary fails,
/// and a page that matches passes.
#[test]
fn a_wrong_example_fails_and_a_right_one_passes() {
    if std::env::var_os("TRYCMD").is_some() {
        return; // overwrite mode would rewrite the wrong page and pass
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let right = scratch.join("right.md");
    std::fs::write(
        &right,
        "```console\n$ zettel init\ninitialized a zettel store at [CWD]\n\n$ zettel check\nno problems found\n\n```\n",
    )
    .unwrap();
    run_page(&scratch, &right);
    let wrong = scratch.join("wrong.md");
    std::fs::write(&wrong, "```console\n$ zettel init\nInitialized\n\n```\n").unwrap();
    assert!(std::panic::catch_unwind(|| run_page(&scratch, &wrong)).is_err());
}

/// Created notes take the pinned prefixes in order, the printed ID shows
/// the pinned prefix, the timestamps and review dates take the fixed
/// values, and a file fence is on disk for the commands after it.
#[test]
fn created_notes_take_pinned_prefixes_and_fixed_times() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("pinned.md");
    std::fs::write(
        &page,
        "```console\n\
         $ zettel init\n\
         initialized a zettel store at [CWD]\n\
         \n\
         $ zettel note create \"First one\" -p agent:summary -b 'A finding.'\n\
         ab12-first-one\n\
         \n\
         $ zettel note create \"Second one\" -l first-one\n\
         cd34-second-one\n\
         \n\
         $ zettel note review first-one --approve all\n\
         1 span(s) approved\n\
         \n\
         $ cat .zettel/ab12-first-one.md\n\
         ---\n\
         title: First one\n\
         provenance: agent:summary reviewed=2026-01-01\n\
         tags: []\n\
         links: []\n\
         created: 2026-01-01T00:00:00Z\n\
         updated: 2026-01-01T00:00:00Z\n\
         ---\n\
         \n\
         A finding.\n\
         \n\
         $ zettel backlinks ab12\n\
         cd34-second-one  Second one\n\
         \n\
         ```\n\
         \n\
         ```text {filename=\"notes/a.txt\"}\n\
         hello\n\
         ```\n\
         \n\
         ```console\n\
         $ cat notes/a.txt\n\
         hello\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// The timestamp rewrite touches only a timestamp value: a body line that
/// starts like a frontmatter key but holds other text stays as written.
#[test]
fn the_timestamp_rewrite_leaves_other_lines_alone() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("body-times.md");
    std::fs::write(
        &page,
        "```console\n\
         $ zettel init\n\
         initialized a zettel store at [CWD]\n\
         \n\
         $ zettel note create Dated -b 'created: 2020-05-05T05:05:05Z in prose'\n\
         ab12-dated\n\
         \n\
         $ zettel note show dated --field body\n\
         created: 2020-05-05T05:05:05Z in prose\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// The commands after a file fence are checked too: a wrong output there
/// fails the page.
#[test]
fn a_wrong_example_after_a_file_fence_fails() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("after.md");
    std::fs::write(
        &page,
        "```text {filename=\"a.txt\"}\nhello\n```\n\n```console\n$ cat a.txt\ngoodbye\n\n```\n",
    )
    .unwrap();
    assert!(std::panic::catch_unwind(|| run_page(&scratch, &page)).is_err());
}

/// A page reads the user config under its own `HOME`, never the one in
/// the passwd home directory. The fixtures in
/// tests/docs/user-config-fixture/ set a `root_store` that no machine
/// has, so the page fails on every machine when the wrapper stops
/// passing `--user-config`. They also prove that `home/` and `work/` are
/// copied.
#[test]
fn a_page_reads_no_machine_user_config() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("user-config-fixture.md");
    std::fs::write(
        &page,
        "```console\n\
         $ zettel store root\n\
         root_store: /docs-test-fixture-root ([..]/user-config-fixture/home/.config/zettel/config.yml)\n\
         \n\
         $ cat note.txt\n\
         from the work fixture\n\
         \n\
         ```\n",
    )
    .unwrap();
    run_page(&scratch, &page);
}

/// A page's commands see none of the test process's environment.
#[test]
fn page_commands_run_in_a_clean_environment() {
    if std::env::var_os("TRYCMD").is_some() {
        return;
    }
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let printenv = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("printenv"))
        .find(|p| p.is_file())
        .expect("printenv on PATH");
    let wrapper = write_wrapper(&scratch, "printenv", &printenv);
    let out = Command::new(&wrapper)
        .env("GIT_DIR", "/elsewhere")
        .env("MDSTORE_REGISTRY", "/elsewhere")
        .env("HOME", "/page-home")
        .output()
        .unwrap();
    let env = String::from_utf8(out.stdout).unwrap();
    assert!(env.contains("HOME=/page-home\n"), "{env}");
    assert!(!env.contains("GIT_DIR"), "{env}");
    assert!(!env.contains("MDSTORE_REGISTRY"), "{env}");
}

#[test]
fn commands_lists_what_trycmd_runs() {
    let page = "```sh\nls\n```\n\n````markdown\n```console\n$ inner\n```\n````\n\n```console\n$ A=1 zettel check\n\n$ git status\nok\n```\n\n```\n$ bare\n```\n\n```trycmd\n$ tagged\n```\n";
    let names: Vec<String> = commands(Path::new("p.md"), page).into_iter().collect();
    assert_eq!(names, ["bare", "git", "tagged", "zettel"]);
}

/// A fence closes only at its own backticks, so a shorter fence inside a
/// longer console block does not end it.
#[test]
fn a_console_block_ends_at_its_own_backticks() {
    let page = "````console\n$ first\n```\n$ second\n````\n";
    let names: Vec<String> = commands(Path::new("p.md"), page).into_iter().collect();
    assert_eq!(names, ["first", "second"]);
}

#[test]
fn a_page_running_an_unknown_command_fails() {
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("unknown.md");
    std::fs::write(&page, "```console\n$ no-such-command-in-path\n```\n").unwrap();
    let err = std::panic::catch_unwind(|| run_page(&scratch, &page)).unwrap_err();
    let message = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("not on PATH"), "{message}");
}

#[test]
#[should_panic(expected = "hides its commands")]
fn an_ignored_console_block_is_refused() {
    commands(
        Path::new("p.md"),
        "```console,ignore\n$ zettel check\n```\n",
    );
}

#[test]
#[should_panic(expected = "backslash")]
fn output_with_a_backslash_is_refused() {
    commands(
        Path::new("p.md"),
        "```console\n$ zettel note show ab12\n\"a\\\"b\"\n```\n",
    );
}

#[test]
#[should_panic(expected = "outside the working directory")]
fn a_file_fence_outside_the_working_directory_is_refused() {
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("escape.md");
    std::fs::write(&page, "```text {filename=\"../x\"}\nx\n```\n").unwrap();
    run_page(&scratch, &page);
}

#[test]
#[should_panic(expected = "outside the working directory")]
fn a_file_fence_with_an_absolute_path_is_refused() {
    let guard = tempfile::tempdir().unwrap();
    let scratch = guard.path().canonicalize().unwrap();
    let page = scratch.join("absolute.md");
    let target = scratch.join("escaped.txt");
    std::fs::write(
        &page,
        format!("```text {{filename=\"{}\"}}\nx\n```\n", target.display()),
    )
    .unwrap();
    run_page(&scratch, &page);
}

#[test]
#[should_panic(expected = "needs a language")]
fn a_file_fence_without_a_language_is_refused() {
    segments(Path::new("p.md"), "```{filename=\"a.txt\"}\nx\n```\n");
}

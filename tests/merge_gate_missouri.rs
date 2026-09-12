//! The merge gate's missouri arm.
//!
//! `tests/merge_gate_guard.rs` sets `MERGE_GATE_SKIP_TESTS`, and cargo
//! sets `CARGO`, so the arm's escape fires and every guard inside it
//! goes unread. Weakening the empty-suite check to `grep -E 'passed'`
//! once left the whole suite green.
//!
//! These tests remove `CARGO` instead, which is what the escape needs,
//! so the arm runs. They drive the real script from a fixture
//! directory against a PATH of stubs, the same shape
//! `tests/prepublish_gate.rs` uses. No cargo build and no missouri run
//! happens. The gate script is copied out of the repository and never
//! runs against it.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A fixture directory holding the real gate script, a `tests/missouri`
/// directory for the arm to find, and a stub PATH. The directory name
/// carries the process id, so two concurrent runs of this suite do not
/// share one path.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("zettel_merge_gate_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).expect("the fixture directory is writable");
        std::fs::create_dir_all(dir.join("tests/missouri")).expect("the suite directory is made");
        std::fs::create_dir_all(dir.join("stubs")).expect("the stub directory is made");

        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/merge-gate.sh");
        let body = std::fs::read_to_string(&script).expect("the gate script is readable");
        std::fs::write(dir.join("scripts/merge-gate.sh"), body).expect("the copy writes");

        // The review-name cross-check runs before the test arms. It
        // needs one required name with criteria beside it, and no
        // vendored name that nothing requires.
        std::fs::create_dir_all(dir.join(".agents/skills/review-tests"))
            .expect("the criteria directory is made");
        std::fs::write(
            dir.join(".agents/skills/review-tests/SKILL.md"),
            "# review-tests\n",
        )
        .expect("the criteria file writes");

        // The cargo arm runs before the missouri arm and must pass, or
        // the missouri arm is never reached. Stubs keep both to one exec.
        Self { dir }
            .stub("gaff", "#!/bin/sh\necho review-tests\n")
            .stub("cargo", "#!/bin/sh\nexit 0\n")
    }

    /// Writes an executable stub onto the fixture's PATH.
    fn stub(self, name: &str, body: &str) -> Self {
        let path = self.dir.join("stubs").join(name);
        std::fs::write(&path, body).expect("the stub writes");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("the stub is executable");
        }
        self
    }

    /// Runs the gate in the fixture. `CARGO` is removed, which is what
    /// turns the escape off and lets the test arms run.
    /// Runs the gate with extra environment on top of the clean set.
    fn run_with_env(&self, extra: &[(&str, &str)]) -> String {
        self.run_inner(extra).1
    }

    fn run(&self) -> (i32, String) {
        self.run_inner(&[])
    }

    fn run_inner(&self, extra: &[(&str, &str)]) -> (i32, String) {
        // The stubs come first, and the system directories follow
        // because the script itself calls `cat`, `grep`, `printf` and
        // `tail`. Neither system directory holds `missouri` or `cargo`,
        // so the stubs stay in control of what this test measures.
        let path = format!("{}:/usr/bin:/bin", self.dir.join("stubs").display());
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("scripts/merge-gate.sh")
            .current_dir(&self.dir)
            .env("PATH", path)
            .env_remove("CARGO")
            .env_remove("CI")
            .env_remove("GITHUB_ACTIONS")
            .env_remove("GITHUB_EVENT_NAME")
            .env_remove("MERGE_GATE_SKIP_TESTS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("merge-gate.sh runs");
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(b"refs/heads/topic 4b825dc refs/heads/main 0000000\n")
            .expect("the ref line writes");
        let out = child.wait_with_output().expect("the gate finishes");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        (out.status.code().unwrap_or(-1), text)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_suite_directory_without_missouri_is_refused() {
    // No missouri stub, so the command is absent from PATH.
    let f = Fixture::new("absent");
    let (code, out) = f.run();
    assert_ne!(code, 0, "an absent missouri passed the gate: {out}");
    assert!(
        out.contains("missouri is not on PATH"),
        "expected the absent-missouri refusal, got: {out}"
    );
}

#[test]
fn a_red_suite_is_refused_and_its_output_is_shown() {
    let f = Fixture::new("red").stub(
        "missouri",
        "#!/bin/sh\necho 'path two: assertion failed'\nexit 1\n",
    );
    let (code, out) = f.run();
    assert_ne!(code, 0, "a red suite passed the gate: {out}");
    assert!(
        out.contains("the missouri suite failed"),
        "expected the red-suite refusal, got: {out}"
    );
    assert!(
        out.contains("path two: assertion failed"),
        "the refusal dropped the suite's own output: {out}"
    );
}

#[test]
fn a_non_zero_exit_is_refused_even_where_the_summary_reads_green() {
    // The exit-code guard and the summary guard both refuse a suite
    // that exits non-zero with no summary, so a test using that stub
    // cannot tell which one fired. Removing the exit-code guard then
    // leaves the suite green.
    //
    // This is the input that separates them: a run that prints a
    // passing summary and then fails. A harness that dies after its
    // last path, or a suite whose runner errors, looks exactly like
    // this. Without the guard the gate prints `merge-gate: ok` and a
    // red missouri suite merges.
    let f = Fixture::new("greensummaryred").stub(
        "missouri",
        "#!/bin/sh\necho '9 passed, 0 failed, 18 steps'\necho 'harness error after the summary'\nexit 1\n",
    );
    let (code, out) = f.run();
    assert_ne!(code, 0, "a red suite with a green summary passed: {out}");
    assert!(
        out.contains("the missouri suite failed"),
        "the refusal did not name the red suite: {out}"
    );
    assert!(
        !out.contains("merge-gate: ok"),
        "the gate reported ok on a red suite: {out}"
    );
}

#[test]
fn the_arm_needs_both_the_marker_and_cargo_to_be_skipped() {
    // The escape is two variables on purpose: a plain shell cannot turn
    // the arm off with one. `MERGE_GATE_SKIP_TESTS` alone must still
    // run it, or a developer's environment silently disables the gate.
    let f = Fixture::new("oneescape").stub(
        "missouri",
        "#!/bin/sh\necho '9 passed, 0 failed, 18 steps'\nexit 0\n",
    );
    let out = f.run_with_env(&[("MERGE_GATE_SKIP_TESTS", "1")]);
    assert!(
        out.contains("merge-gate: missouri run"),
        "one variable turned the arm off: {out}"
    );
}

#[test]
fn a_suite_that_reports_no_passing_path_is_refused() {
    // This is the guard a mutation defeated. The command exits 0, and
    // the summary says nothing passed. An empty suite gates nothing.
    let f = Fixture::new("empty").stub(
        "missouri",
        "#!/bin/sh\necho '0 passed, 0 failed, 0 steps'\nexit 0\n",
    );
    let (code, out) = f.run();
    assert_ne!(code, 0, "an empty suite passed the gate: {out}");
    assert!(
        out.contains("no passing path"),
        "expected the empty-suite refusal, got: {out}"
    );
}

#[test]
fn a_failure_count_above_zero_is_refused_for_the_failures() {
    // The summary must read zero failures. One message served this and
    // the empty case both, and it told a developer looking at two
    // failed paths that an empty suite gates nothing, which sends them
    // to write a test rather than fix the two. The message must name
    // the state the reader is actually in.
    let f = Fixture::new("failures").stub(
        "missouri",
        "#!/bin/sh\necho '7 passed, 2 failed, 30 steps'\nexit 0\n",
    );
    let (code, out) = f.run();
    assert_ne!(code, 0, "a suite reporting failures passed the gate: {out}");
    assert!(
        out.contains("reported failures"),
        "the refusal did not name the failures: {out}"
    );
    assert!(
        !out.contains("no passing path"),
        "a failing suite was told it was empty: {out}"
    );
    assert!(
        out.contains("7 passed, 2 failed"),
        "the refusal did not show the summary it rejected: {out}"
    );
}

#[test]
fn a_suite_that_prints_no_summary_is_refused_for_that() {
    // The third state. A run that exits 0 and prints nothing the gate
    // recognises has not been shown to have run at all.
    let f = Fixture::new("nosummary").stub(
        "missouri",
        "#!/bin/sh\necho 'some other output entirely'\nexit 0\n",
    );
    let (code, out) = f.run();
    assert_ne!(code, 0, "a suite with no summary passed the gate: {out}");
    assert!(
        out.contains("no summary line"),
        "the refusal did not name the missing summary: {out}"
    );
}

#[test]
fn a_green_suite_passes_the_missouri_arm() {
    // The arm must admit a real summary, and the gate must then reach
    // its end. The `gaff` stub exits 0 for every argument, `reviews
    // check` included, so the whole script runs and the exit code is
    // the assertion worth making. Two absent strings would pass a run
    // that refused somewhere else.
    let f = Fixture::new("green").stub(
        "missouri",
        "#!/bin/sh\necho '9 passed, 0 failed, 18 steps'\nexit 0\n",
    );
    let (code, out) = f.run();
    assert!(
        out.contains("merge-gate: missouri run"),
        "the missouri arm never ran, so this suite proves nothing: {out}"
    );
    assert_eq!(
        code, 0,
        "a green suite did not reach the end of the gate: {out}"
    );
    assert!(
        out.contains("merge-gate: ok"),
        "the gate did not report ok: {out}"
    );
}

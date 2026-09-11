//! The CI review gate's shell, run. `.github/workflows/review-gate.yml`
//! carries its step inline, and nothing else executed it: a version
//! that never compared the sign-off's sha with the head passed a line
//! copied from an older note. This extracts the step and drives it
//! against a scratch repository, once per refusal a note can earn.
//!
//! The step hands the note to `gaff reviews check`, so gaff is on PATH
//! here as it is in CI. The refusals are gaff's; the test reads that
//! the step reaches each one and that its own two refusals hold.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `run:` block of the named step, dedented.
fn step_script(name: &str) -> String {
    let src = std::fs::read_to_string(".github/workflows/review-gate.yml").expect("workflow");
    let mut lines = src.lines();
    lines
        .by_ref()
        .find(|l| l.contains(&format!("name: '{name}'")))
        .unwrap_or_else(|| panic!("no step named {name}"));
    let run = lines
        .by_ref()
        .find(|l| l.trim() == "run: |")
        .expect("the step has a run: block");
    let indent = run.len() - run.trim_start().len() + 2;
    let mut script = String::new();
    for line in lines {
        if !line.trim().is_empty() && line.len() - line.trim_start().len() < indent {
            break;
        }
        script.push_str(line.get(indent..).unwrap_or(""));
        script.push('\n');
    }
    assert!(
        script.contains("gaff reviews check"),
        "the extracted script does not call gaff:\n{script}"
    );
    script
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository declaring two reviews and holding one commit. Returns
/// the repository and its head sha.
fn scratch_repo(name: &str) -> (PathBuf, String) {
    let repo = std::env::temp_dir().join(format!("zettel_review_gate_{name}"));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join(".gaff")).expect("scratch repo");
    std::fs::write(
        repo.join(".gaff/gaff.yml"),
        "reviews:\n  - review-a\n  - review-b\n",
    )
    .expect("policy");
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "one"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, head)
}

/// Run the extracted step with `HEAD_SHA` set to `head`. Returns
/// (output, exit code).
fn run_gate(repo: &Path, head: &str) -> (String, i32) {
    let script = repo.join("check.sh");
    std::fs::write(
        &script,
        step_script("every declared review signed off on the head commit"),
    )
    .expect("write script");
    let out = Command::new("sh")
        .arg(&script)
        .current_dir(repo)
        .env("HEAD_SHA", head)
        .output()
        .expect("sh runs");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code().unwrap_or(-1),
    )
}

/// Write a note on the head, then run the gate against it.
fn gate(repo: &Path, head: &str, note: &str) -> (String, i32) {
    git(
        repo,
        &["notes", "--ref=reviews", "add", "-f", "-m", note, head],
    );
    run_gate(repo, head)
}

#[test]
fn a_bound_note_with_evidence_passes() {
    let (repo, head) = scratch_repo("passes");
    let short = &head[..7];
    let note = format!(
        "signoff[review-a] PASS {short} read every guard and ran the suite\n\
         signoff[review-b] PASS {head} removed a check, one test went red\n"
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("review-gate: ok"), "{out}");
}

#[test]
fn a_line_naming_another_commit_is_refused() {
    let (repo, head) = scratch_repo("other_commit");
    let short = &head[..7];
    let note = format!(
        "signoff[review-a] PASS {short} read every guard and ran the suite\n\
         signoff[review-b] PASS 0123abc removed a check, one test went red\n"
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "a sign-off for another commit passed: {out}");
    assert!(
        out.contains("review-b") && out.contains("different commit"),
        "{out}"
    );
}

#[test]
fn thin_evidence_is_refused() {
    let (repo, head) = scratch_repo("thin");
    let short = &head[..7];
    let note = format!(
        "signoff[review-a] PASS {short} read every guard and ran the suite\n\
         signoff[review-b] PASS {short} looks fine\n"
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "two words of evidence passed: {out}");
    assert!(out.contains("words of evidence"), "{out}");
}

#[test]
fn a_sha_under_seven_characters_is_refused() {
    // Six characters match too many commits; the floor is seven.
    let (repo, head) = scratch_repo("short_sha");
    let short = &head[..7];
    let note = format!(
        "signoff[review-a] PASS {short} read every guard and ran the suite\n\
         signoff[review-b] PASS {} removed a check, one test went red\n",
        &head[..6]
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "a six-character sha passed: {out}");
    assert!(out.contains("hex characters"), "{out}");
}

#[test]
fn two_lines_for_one_review_are_refused() {
    let (repo, head) = scratch_repo("duplicate");
    let note = format!(
        "signoff[review-a] PASS {head} read every guard and ran the suite\n\
         signoff[review-a] PASS {head} read it again on a second pass\n\
         signoff[review-b] PASS {head} removed a check, one test went red\n"
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "two lines for one review passed: {out}");
    assert!(out.contains("two review-a sign-offs"), "{out}");
}

#[test]
fn an_empty_policy_requires_nothing() {
    // `reviews: []` is the one path that turns the gate off with a
    // success. An earlier form printed its own hint and exited 1.
    let (repo, head) = scratch_repo("empty_policy");
    std::fs::write(repo.join(".gaff/gaff.yml"), "reviews: []\n").expect("policy");
    let (out, code) = gate(&repo, &head, "no lines at all\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("requires no review"), "{out}");
}

#[test]
fn a_policy_with_no_reviews_key_is_refused() {
    // Absent is not empty. gaff refuses the policy and names the fix.
    let (repo, head) = scratch_repo("no_policy");
    std::fs::write(repo.join(".gaff/gaff.yml"), "reminders: []\n").expect("policy");
    let (out, code) = gate(&repo, &head, "no lines at all\n");
    assert_ne!(code, 0, "a policy with no reviews key passed: {out}");
    assert!(out.contains("reviews"), "{out}");
}

#[test]
fn a_comment_inside_the_list_drops_no_review() {
    // An earlier form parsed the list itself and stopped at the first
    // line that was not an entry, so a comment between two entries
    // dropped every review after it and the gate announced enforcement.
    let (repo, head) = scratch_repo("comment_in_list");
    std::fs::write(
        repo.join(".gaff/gaff.yml"),
        "reviews:\n  - review-a\n  # review-b reads the tests\n  - review-b\n",
    )
    .expect("policy");
    let note = format!("signoff[review-a] PASS {head} read every guard and ran the suite\n");
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "a review after a comment went unchecked: {out}");
    assert!(
        out.contains("no sign-off for") && out.contains("review-b"),
        "{out}"
    );
}

#[test]
fn a_missing_review_is_refused_by_name() {
    let (repo, head) = scratch_repo("missing");
    let note = format!("signoff[review-a] PASS {head} read every guard and ran the suite\n");
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "{out}");
    assert!(
        out.contains("no sign-off for") && out.contains("review-b"),
        "{out}"
    );
    assert!(
        out.contains("git notes --ref=reviews add -m '<the lines>'"),
        "the refusal names the one write that records every line: {out}"
    );
}

#[test]
fn a_fail_verdict_is_refused() {
    let (repo, head) = scratch_repo("fail");
    let note = format!(
        "signoff[review-a] PASS {head} read every guard and ran the suite\n\
         signoff[review-b] FAIL {head} blocking: the guard has no test\n"
    );
    let (out, code) = gate(&repo, &head, &note);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("failed") && out.contains("review-b"), "{out}");
}

#[test]
fn a_head_with_no_note_is_refused() {
    let (repo, head) = scratch_repo("no_note");
    let (out, code) = run_gate(&repo, &head);
    assert_ne!(code, 0, "a head with no note passed: {out}");
    assert!(
        out.contains("no review note") && out.contains("refs/notes/reviews"),
        "{out}"
    );
}

#[test]
fn an_unreadable_head_sha_is_refused() {
    // The sha comes from the event payload. A value that is not hex
    // reaches no git command and no gaff command.
    let (repo, _head) = scratch_repo("bad_sha");
    let (out, code) = run_gate(&repo, "not-a-sha");
    assert_ne!(code, 0, "an unreadable head sha passed: {out}");
    assert!(out.contains("unreadable"), "{out}");
}

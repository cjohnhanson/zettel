//! The CI review gate's shell, run. `.github/workflows/review-gate.yml`
//! carries its check inline, and nothing else executed it: a version
//! that never compared the sign-off's sha with the head passed a line
//! copied from an older note. This extracts the step and drives it
//! against a scratch repository.

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
        script.contains("signoff"),
        "the extracted script reads no sign-off:\n{script}"
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

/// A repository declaring two reviews, with criteria for both, and one
/// commit. Returns the repository and its head sha.
fn scratch_repo(name: &str) -> (PathBuf, String) {
    let repo = std::env::temp_dir().join(format!("zettel_review_gate_{name}"));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join(".gaff")).expect("scratch repo");
    std::fs::write(
        repo.join(".gaff/gaff.yml"),
        "reviews:\n  - review-a\n  - review-b\n",
    )
    .expect("policy");
    for r in ["review-a", "review-b"] {
        let dir = repo.join(".agents/skills").join(r);
        std::fs::create_dir_all(&dir).expect("criteria dir");
        std::fs::write(dir.join("SKILL.md"), "# criteria\n").expect("criteria");
    }
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "one"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, head)
}

/// Run the gate against a note on the head. Returns (output, exit code).
fn gate(repo: &Path, head: &str, note: &str) -> (String, i32) {
    git(
        repo,
        &["notes", "--ref=reviews", "add", "-f", "-m", note, head],
    );
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
        out.contains("review-b") && out.contains("another commit"),
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
    assert!(out.contains("FAIL"), "{out}");
}

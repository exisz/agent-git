//! Integration tests for `agent-git clone` flag passthrough (PLANET-1527).
//!
//! Upstream tools (e.g. peaceiris/actions-gh-pages@v4) call
//! `git clone --depth 1 --branch <b> --single-branch <url> <dst>`. Because
//! agent-git is installed PATH-front as the `git` shim on agent hosts,
//! that invocation is routed through `agent-git clone`. Before this fix,
//! clap rejected the unknown `--depth` flag with `error: unexpected argument
//! '--depth' found`. We now accept (and forward) any extra flags after
//! `clone` straight to the underlying git binary while preserving
//! duplicate-detection and banned-path guards.
//!
//! These tests use a per-test HOME so the registry is isolated. They hit
//! the public github.com/octocat/Hello-World repo. Skipped automatically
//! when `AGENT_GIT_NO_NETWORK=1` is set.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_agent-git")
}

fn skip_if_offline() -> bool {
    std::env::var("AGENT_GIT_NO_NETWORK").ok().as_deref() == Some("1")
}

/// Run `agent-git clone <args...>` with HOME pointed at `home_dir` so the
/// registry is isolated from the developer's real ~/.agentgit.
fn run_clone(home_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .arg("clone")
        .args(args)
        .env("HOME", home_dir)
        // Make sure the wrapper finds /usr/bin/git (the real one), not itself.
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .expect("failed to spawn agent-git")
    }

#[test]
fn clone_no_flags_still_works() {
    if skip_if_offline() {
        return;
    }
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    let dst = work.path().join("hello-noflag");
    let out = run_clone(
        home.path(),
        &[
            "https://github.com/octocat/Hello-World.git",
            dst.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "clone (no flags) failed: status={:?}\nstdout={}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dst.join(".git").exists(), "{} missing .git", dst.display());
}

#[test]
fn clone_passthrough_depth_branch() {
    if skip_if_offline() {
        return;
    }
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    let dst = work.path().join("hello-shallow");
    let out = run_clone(
        home.path(),
        &[
            "--depth",
            "1",
            "--branch",
            "master",
            "https://github.com/octocat/Hello-World.git",
            dst.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "clone --depth 1 --branch master failed: status={:?}\nstdout={}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dst.join(".git").exists(), "missing .git after shallow clone");

    // Verify shallow: only 1 commit reachable from HEAD.
    let log = Command::new("/usr/bin/git")
        .args(["-C", dst.to_str().unwrap(), "rev-list", "--count", "HEAD"])
        .output()
        .expect("git rev-list");
    assert!(log.status.success(), "rev-list failed");
    let n: u32 = String::from_utf8_lossy(&log.stdout)
        .trim()
        .parse()
        .expect("commit count");
    assert_eq!(n, 1, "expected exactly 1 commit on shallow HEAD, got {n}");

    // Verify branch.
    let head = Command::new("/usr/bin/git")
        .args(["-C", dst.to_str().unwrap(), "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("git rev-parse");
    assert_eq!(
        String::from_utf8_lossy(&head.stdout).trim(),
        "master",
        "expected HEAD on master"
    );
}

#[test]
fn clone_passthrough_single_branch() {
    if skip_if_offline() {
        return;
    }
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    let dst = work.path().join("hello-single");
    let out = run_clone(
        home.path(),
        &[
            "--single-branch",
            "--branch",
            "master",
            "https://github.com/octocat/Hello-World.git",
            dst.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "clone --single-branch failed: status={:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dst.join(".git").exists());
}

#[test]
fn clone_passthrough_filter_blob_none() {
    if skip_if_offline() {
        return;
    }
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    let dst = work.path().join("hello-filter");
    let out = run_clone(
        home.path(),
        &[
            "--filter=blob:none",
            "https://github.com/octocat/Hello-World.git",
            dst.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "clone --filter=blob:none failed: status={:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dst.join(".git").exists());
}

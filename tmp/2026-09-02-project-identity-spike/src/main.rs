//! T01 spike, part 2 — canonicalizer check for R1.6.
//!
//! R1.6 (opencode dashboard requirements doc): project identity = the
//! canonical git repository toplevel path of a session's working directory;
//! if that directory isn't inside a git repo, fall back to the canonicalized
//! working directory itself. Canonicalize (resolve symlinks, strip trailing
//! slash, normalize case) before comparing paths. Two git worktrees of the
//! same repo must resolve to two DIFFERENT identities (no worktree-merging).
//!
//! This binary implements exactly that resolver and runs it against 5 real
//! filesystem fixtures, printing the observed output for each so the results
//! can be pasted into the evidence report verbatim. `cargo test` covers the
//! same fixtures as assertions.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolves R1.6's "project identity" for `dir`: the canonical git
/// repository toplevel path, or the canonicalized `dir` itself if `dir`
/// isn't inside a git repo.
///
/// Canonicalization is `std::fs::canonicalize` — it resolves symlinks in the
/// path, requires the path to exist, and (per the stdlib docs) returns an
/// absolute path with `.`/`..` and any trailing slash resolved away. It does
/// NOT normalize case on a case-insensitive-but-case-preserving filesystem
/// (e.g. default macOS APFS) — see the report's check 2.5 note.
///
/// Applied twice: once to `dir` up front (so the git lookup itself runs
/// against a canonical path, and so the non-git fallback is canonical), and
/// once to git's own `--show-toplevel` answer (macOS's `/tmp` ->
/// `/private/tmp` symlink means git's answer can itself need
/// re-canonicalizing — confirmed necessary by check 2.1 below).
pub fn resolve_project_identity(dir: &Path) -> std::io::Result<PathBuf> {
    let canon_dir = std::fs::canonicalize(dir)?;

    let git_result = Command::new("git")
        .arg("-C")
        .arg(&canon_dir)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output();

    if let Ok(output) = git_result {
        if output.status.success() {
            let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !toplevel.is_empty() {
                return std::fs::canonicalize(&toplevel);
            }
        }
    }

    Ok(canon_dir)
}

/// This repo's root, as an absolute path. Hardcoded rather than derived from
/// `CARGO_MANIFEST_DIR` — this spike is evidence for one specific repo's
/// requirements doc, not a portable tool.
const REPO_ROOT: &str = "/Users/ajeesh/projects/madaboutcode/opencode-mcp";

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest_dir.join("fixtures");
    std::fs::create_dir_all(&fixtures).expect("create fixtures dir");

    println!("=== R1.6 canonicalizer check — part 2 ===\n");

    // Check 2.1 — plain non-git temp directory. Must live OUTSIDE this
    // repo's working tree (`fixtures/` is a subdirectory of this git repo,
    // so a fixture placed there is not actually "non-git" — `git
    // rev-parse --show-toplevel` would find this repo above it and give a
    // false pass). `std::env::temp_dir()` (macOS: under `/var/folders/...`)
    // is not inside any git repo.
    let nogit_dir = std::env::temp_dir().join(format!(
        "project-identity-spike-nogit-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&nogit_dir).expect("create nogit fixture");
    report("2.1 non-git temp dir", &nogit_dir);
    let _ = std::fs::remove_dir_all(&nogit_dir); // tidy up; outside repo, not load-bearing

    // Check 2.2 — this repo's root.
    let repo_root = Path::new(REPO_ROOT);
    report("2.2 repo root", repo_root);

    // Check 2.3 — subfolder inside this repo.
    let subfolder = repo_root.join("docs/internal");
    report("2.3 repo subfolder (docs/internal)", &subfolder);

    // Check 2.4 — symlink pointing at this repo's root.
    let symlink_path = fixtures.join("symlink-to-repo-root");
    let _ = std::fs::remove_file(&symlink_path); // idempotent re-run
    #[cfg(unix)]
    std::os::unix::fs::symlink(repo_root, &symlink_path).expect("create symlink fixture");
    report("2.4 symlink -> repo root", &symlink_path);

    // Check 2.5 — two separate git worktree checkouts of this repo.
    let worktree_a = fixtures.join("worktree-a");
    let worktree_b = fixtures.join("worktree-b");
    for wt in [&worktree_a, &worktree_b] {
        if wt.exists() {
            let _ = Command::new("git")
                .arg("-C")
                .arg(REPO_ROOT)
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(wt)
                .status();
        }
    }
    for wt in [&worktree_a, &worktree_b] {
        let status = Command::new("git")
            .arg("-C")
            .arg(REPO_ROOT)
            .arg("worktree")
            .arg("add")
            .arg("--detach")
            .arg(wt)
            .arg("HEAD")
            .status()
            .expect("run git worktree add");
        assert!(status.success(), "git worktree add failed for {wt:?}");
    }
    report("2.5a worktree A", &worktree_a);
    report("2.5b worktree B", &worktree_b);

    let id_a = resolve_project_identity(&worktree_a).expect("resolve worktree A");
    let id_b = resolve_project_identity(&worktree_b).expect("resolve worktree B");
    println!(
        "\n2.5 worktrees distinct identities: {} (A={:?}, B={:?})",
        id_a != id_b,
        id_a,
        id_b
    );

    // Clean up worktrees so nothing registered is left behind, per the task
    // contract's explicit instruction.
    for wt in [&worktree_a, &worktree_b] {
        let status = Command::new("git")
            .arg("-C")
            .arg(REPO_ROOT)
            .arg("worktree")
            .arg("remove")
            .arg(wt)
            .status()
            .expect("run git worktree remove");
        assert!(status.success(), "git worktree remove failed for {wt:?}");
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(REPO_ROOT)
        .arg("worktree")
        .arg("prune")
        .status()
        .expect("run git worktree prune");
    assert!(status.success(), "git worktree prune failed");
    println!("\nworktrees removed and pruned.");
}

fn report(label: &str, dir: &Path) {
    match resolve_project_identity(dir) {
        Ok(identity) => println!("{label}\n  input:    {dir:?}\n  resolved: {identity:?}\n"),
        Err(e) => println!("{label}\n  input:    {dir:?}\n  ERROR:    {e}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    #[test]
    fn non_git_dir_falls_back_to_canonicalized_self() {
        // Must live outside this repo's working tree — see the comment on
        // the equivalent fixture in `main`.
        let dir = std::env::temp_dir().join(format!(
            "project-identity-spike-test-nogit-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let identity = resolve_project_identity(&dir).unwrap();
        let expected = std::fs::canonicalize(&dir).unwrap();
        assert_eq!(identity, expected);
    }

    #[test]
    fn repo_root_resolves_to_itself() {
        let identity = resolve_project_identity(Path::new(REPO_ROOT)).unwrap();
        assert_eq!(identity, std::fs::canonicalize(REPO_ROOT).unwrap());
    }

    #[test]
    fn subfolder_resolves_to_repo_root() {
        let subfolder = Path::new(REPO_ROOT).join("docs/internal");
        let identity_sub = resolve_project_identity(&subfolder).unwrap();
        let identity_root = resolve_project_identity(Path::new(REPO_ROOT)).unwrap();
        assert_eq!(identity_sub, identity_root);
    }

    #[test]
    #[cfg(unix)]
    fn symlink_to_repo_root_resolves_to_same_identity() {
        let symlink_path = fixtures_dir().join("test-symlink-to-repo-root");
        let _ = std::fs::remove_file(&symlink_path);
        std::os::unix::fs::symlink(REPO_ROOT, &symlink_path).unwrap();
        let identity_via_symlink = resolve_project_identity(&symlink_path).unwrap();
        let identity_root = resolve_project_identity(Path::new(REPO_ROOT)).unwrap();
        assert_eq!(identity_via_symlink, identity_root);
    }

    #[test]
    fn two_worktrees_of_same_repo_get_distinct_identities() {
        let dir = fixtures_dir();
        let worktree_a = dir.join("test-worktree-a");
        let worktree_b = dir.join("test-worktree-b");
        for wt in [&worktree_a, &worktree_b] {
            if wt.exists() {
                Command::new("git")
                    .args(["-C", REPO_ROOT, "worktree", "remove", "--force"])
                    .arg(wt)
                    .status()
                    .unwrap();
            }
        }
        for wt in [&worktree_a, &worktree_b] {
            let status = Command::new("git")
                .args(["-C", REPO_ROOT, "worktree", "add", "--detach"])
                .arg(wt)
                .arg("HEAD")
                .status()
                .unwrap();
            assert!(status.success());
        }

        let id_a = resolve_project_identity(&worktree_a).unwrap();
        let id_b = resolve_project_identity(&worktree_b).unwrap();
        assert_ne!(
            id_a, id_b,
            "two worktrees must get separate project identities"
        );

        for wt in [&worktree_a, &worktree_b] {
            let status = Command::new("git")
                .args(["-C", REPO_ROOT, "worktree", "remove"])
                .arg(wt)
                .status()
                .unwrap();
            assert!(status.success());
        }
        let status = Command::new("git")
            .args(["-C", REPO_ROOT, "worktree", "prune"])
            .status()
            .unwrap();
        assert!(status.success());
    }
}

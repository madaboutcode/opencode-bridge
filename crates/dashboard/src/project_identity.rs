//! Project identity resolution — `docs/specs/dashboard/client.md` R1.6.
//!
//! The canonicalization logic in [`GitDirResolver::resolve`] is ported
//! verbatim from the already-verified T01 spike
//! (`tmp/2026-09-02-project-identity-spike/src/main.rs`,
//! `resolve_project_identity`), per the T09 contract's explicit instruction
//! to re-derive it from that evidence rather than redesign it from the spec
//! text. The 9 real-filesystem checks in that spike's `EVIDENCE.md` (repo
//! root, subfolder, symlink, two worktrees, explicit bridge `directory`
//! param, non-git temp dir) all matched R1.6 as written; the fixture tests
//! below re-run the same checks against this ported copy.
//!
//! This module is harness-agnostic: project identity is a pure function of
//! a working directory, the same for any harness. It lives alongside
//! `adapter.rs`/`snapshot.rs`, not under `opencode/`.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::snapshot::{ProjectId, SessionId};

/// Seam over "turn a directory into its canonical project-identity path" so
/// the caching layer below (`ProjectIdentityCache`) can be tested without
/// actually spawning `git` for every call — see the caching-obligation test
/// at the bottom of this file. `GitDirResolver` is the one real
/// implementation; any other implementation exists only in tests.
pub trait DirResolver {
    fn resolve(&self, dir: &Path) -> io::Result<PathBuf>;
}

/// The real resolver: `client.md` R1.6, ported from the T01 spike.
#[derive(Debug, Default, Clone, Copy)]
pub struct GitDirResolver;

impl DirResolver for GitDirResolver {
    /// Canonicalize `dir` (resolves symlinks, strips trailing slash,
    /// requires the path to exist), run `git -C <canon_dir> rev-parse
    /// --show-toplevel`, and if that succeeds, canonicalize ITS output too
    /// (needed — macOS's `/tmp` -> `/private/tmp` symlink means git's own
    /// answer can need re-canonicalizing, confirmed by the spike's check
    /// 2.1) and return it; otherwise return the canonicalized input.
    fn resolve(&self, dir: &Path) -> io::Result<PathBuf> {
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
}

/// Caches a session's directory→project-identity mapping for that session's
/// whole lifetime (`client.md` R1.6's caching obligation): resolving to git
/// toplevel spawns a subprocess, and a session's directory can't change
/// during its own lifetime, so the resolver must not re-spawn `git` on
/// every snapshot or redraw. Cached per `SessionId`, per the contract's
/// explicit instruction (a directory-keyed cache would also be correct and
/// would share hits across sessions in the same project, but the contract
/// asks for per-session caching and that's what this implements).
#[derive(Default)]
pub struct ProjectIdentityCache<R: DirResolver = GitDirResolver> {
    resolver: R,
    cache: HashMap<SessionId, ProjectId>,
}

impl<R: DirResolver> ProjectIdentityCache<R> {
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            cache: HashMap::new(),
        }
    }

    /// Resolves `dir`'s project identity for `session`, spawning `git` at
    /// most once across this cache's lifetime for a given session.
    pub fn resolve(&mut self, session: &SessionId, dir: &Path) -> io::Result<ProjectId> {
        if let Some(id) = self.cache.get(session) {
            return Ok(id.clone());
        }
        let resolved = self.resolver.resolve(dir)?;
        let id = ProjectId::from_canonical(resolved);
        self.cache.insert(session.clone(), id.clone());
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::HarnessKind;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const TEST_KIND: HarnessKind = HarnessKind("test");

    /// This repo's own root, discovered dynamically (never hardcoded — the
    /// spike hardcoded its own machine's path since it was throwaway
    /// evidence; production code and its tests must not).
    fn repo_root() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let output = Command::new("git")
            .arg("-C")
            .arg(manifest_dir)
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output()
            .expect("run git rev-parse --show-toplevel");
        assert!(output.status.success(), "git rev-parse failed");
        let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
        std::fs::canonicalize(toplevel).expect("canonicalize repo root")
    }

    // --- Correctness fixtures, ported from the T01 spike's part 2 ---

    #[test]
    fn non_git_dir_falls_back_to_canonicalized_self() {
        let dir = std::env::temp_dir().join(format!(
            "dashboard-project-identity-test-nogit-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let resolved = GitDirResolver.resolve(&dir).unwrap();
        let expected = std::fs::canonicalize(&dir).unwrap();
        assert_eq!(resolved, expected);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repo_root_resolves_to_itself() {
        let root = repo_root();
        let resolved = GitDirResolver.resolve(&root).unwrap();
        assert_eq!(resolved, root);
    }

    #[test]
    fn subfolder_resolves_to_repo_root() {
        let root = repo_root();
        let subfolder = root.join("docs/internal");
        let resolved_sub = GitDirResolver.resolve(&subfolder).unwrap();
        let resolved_root = GitDirResolver.resolve(&root).unwrap();
        assert_eq!(resolved_sub, resolved_root);
    }

    #[test]
    #[cfg(unix)]
    fn symlink_to_repo_root_resolves_to_same_identity() {
        let root = repo_root();
        let symlink_path = std::env::temp_dir().join(format!(
            "dashboard-project-identity-test-symlink-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&symlink_path);
        std::os::unix::fs::symlink(&root, &symlink_path).unwrap();
        let resolved_via_symlink = GitDirResolver.resolve(&symlink_path).unwrap();
        let resolved_root = GitDirResolver.resolve(&root).unwrap();
        assert_eq!(resolved_via_symlink, resolved_root);
        let _ = std::fs::remove_file(&symlink_path);
    }

    /// Cleans up its worktrees on drop, even if an assertion panics —
    /// unlike the spike's fire-and-forget cleanup, a failed assertion here
    /// must not leave a registered worktree behind.
    struct WorktreeGuard {
        root: PathBuf,
        paths: Vec<PathBuf>,
    }

    impl Drop for WorktreeGuard {
        fn drop(&mut self) {
            for wt in &self.paths {
                let _ = Command::new("git")
                    .args(["-C"])
                    .arg(&self.root)
                    .args(["worktree", "remove", "--force"])
                    .arg(wt)
                    .status();
            }
            let _ = Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["worktree", "prune"])
                .status();
        }
    }

    #[test]
    fn two_worktrees_of_same_repo_get_distinct_identities() {
        let root = repo_root();
        let base = std::env::temp_dir().join(format!(
            "dashboard-project-identity-test-worktrees-{}",
            std::process::id()
        ));
        let worktree_a = base.join("a");
        let worktree_b = base.join("b");
        let _guard = WorktreeGuard {
            root: root.clone(),
            paths: vec![worktree_a.clone(), worktree_b.clone()],
        };

        for wt in [&worktree_a, &worktree_b] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["worktree", "add", "--detach"])
                .arg(wt)
                .arg("HEAD")
                .status()
                .unwrap();
            assert!(status.success(), "git worktree add failed for {wt:?}");
        }

        let id_a = GitDirResolver.resolve(&worktree_a).unwrap();
        let id_b = GitDirResolver.resolve(&worktree_b).unwrap();
        assert_ne!(
            id_a, id_b,
            "two worktrees of the same repo must resolve to distinct identities"
        );
        assert_ne!(
            id_a, root,
            "a worktree must not merge into the main checkout"
        );
    }

    // --- Caching-obligation proof (AC3): resolver isn't re-spawned per call ---

    #[derive(Clone)]
    struct CountingResolver {
        calls: Arc<AtomicUsize>,
        answer: PathBuf,
    }

    impl DirResolver for CountingResolver {
        fn resolve(&self, _dir: &Path) -> io::Result<PathBuf> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.answer.clone())
        }
    }

    #[test]
    fn cache_resolves_a_given_session_at_most_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let resolver = CountingResolver {
            calls: calls.clone(),
            answer: PathBuf::from("/tmp/whatever-project"),
        };
        let mut cache = ProjectIdentityCache::new(resolver);
        let session = SessionId::new(TEST_KIND, "ses_1");
        let dir = Path::new("/tmp/session-1-cwd");

        let first = cache.resolve(&session, dir).unwrap();
        let second = cache.resolve(&session, dir).unwrap();
        let third = cache.resolve(&session, dir).unwrap();

        assert_eq!(first, second);
        assert_eq!(second, third);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "resolving the same session repeatedly must spawn the underlying resolver once, not once per call"
        );
    }

    #[test]
    fn cache_resolves_different_sessions_independently() {
        let calls = Arc::new(AtomicUsize::new(0));
        let resolver = CountingResolver {
            calls: calls.clone(),
            answer: PathBuf::from("/tmp/whatever-project"),
        };
        let mut cache = ProjectIdentityCache::new(resolver);
        let session_a = SessionId::new(TEST_KIND, "ses_a");
        let session_b = SessionId::new(TEST_KIND, "ses_b");
        let dir = Path::new("/tmp/shared-cwd");

        cache.resolve(&session_a, dir).unwrap();
        cache.resolve(&session_b, dir).unwrap();

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "two distinct sessions must each resolve once, even sharing a directory"
        );
    }
}

//! Small process/filesystem helpers shared by xtask commands.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Error type for xtask commands: a plain message is enough for a CLI tool
/// whose only consumer is a human reading stderr.
pub type Res<T> = Result<T, String>;

/// Repo root — the directory containing this xtask crate's parent.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives directly under the repo root")
        .to_path_buf()
}

/// Run a command from the repo root, streaming its output live, and return
/// an error carrying the command line if it exits non-zero or fails to spawn.
pub fn run(program: &str, args: &[&str]) -> Res<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(repo_root())
        .status()
        .map_err(|e| format!("failed to spawn `{program} {}`: {e}", args.join(" ")))?;
    if !status.success() {
        return Err(format!(
            "`{program} {}` exited with {status}",
            args.join(" ")
        ));
    }
    Ok(())
}

/// Like [`run`], but returns whether the command succeeded instead of an
/// error — used for preflight checks where a non-zero exit just means
/// "not installed", not a real failure worth aborting on.
pub fn run_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .current_dir(repo_root())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Capture a command's stdout as UTF-8 (lossily), from the repo root.
/// Returns `None` if the command fails to spawn or exits non-zero.
pub fn capture(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .current_dir(repo_root())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// SHA-256 of a file's contents, as a lowercase hex string.
pub fn sha256_file(path: &Path) -> Res<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

//! Shared helpers for xtask commands: process execution rooted at the repo,
//! path resolution, SHA-256, checksum verification, and a localhost health poll.

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

pub type Res<T> = Result<T, Box<dyn Error>>;

/// The repository root, derived from this crate's location (`<root>/xtask`), so
/// the tool is independent of the caller's working directory.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate always has a parent (the repo root)")
        .to_path_buf()
}

/// Resolve a possibly-relative path against the repo root. All xtask commands
/// treat paths as relative to the repo root (matching the scripts' "run from
/// the repo root" convention).
pub fn abs(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo_root().join(p)
    }
}

/// Build a `Command` rooted at the repo. On Windows `npm`/`npx` are `.cmd`
/// shims that `Command::new` won't find, so they run through `cmd /C`.
fn command(program: &str, args: &[&str]) -> Command {
    let root = repo_root();
    if cfg!(windows) && (program == "npm" || program == "npx") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(program).args(args).current_dir(root);
        c
    } else {
        let mut c = Command::new(program);
        c.args(args).current_dir(root);
        c
    }
}

/// Run a command from the repo root, streaming its output; error on failure.
pub fn run(program: &str, args: &[&str]) -> Res<()> {
    println!("== {program} {} ==", args.join(" "));
    let status = command(program, args).status()?;
    if !status.success() {
        return Err(format!("`{program}` failed ({status})").into());
    }
    Ok(())
}

/// Run a command from a specific sub-directory of the repo (e.g. `frontend`),
/// streaming output; error on failure. Used for tools like Playwright whose
/// config resolves relative to the package directory.
pub fn run_in(subdir: &str, program: &str, args: &[&str]) -> Res<()> {
    println!("== ({subdir}) {program} {} ==", args.join(" "));
    let dir = repo_root().join(subdir);
    let status = if cfg!(windows) && (program == "npm" || program == "npx") {
        Command::new("cmd")
            .arg("/C")
            .arg(program)
            .args(args)
            .current_dir(dir)
            .status()?
    } else {
        Command::new(program).args(args).current_dir(dir).status()?
    };
    if !status.success() {
        return Err(format!("`{program}` failed ({status})").into());
    }
    Ok(())
}

/// Run a command capturing trimmed stdout; error on failure.
pub fn run_capture(program: &str, args: &[&str]) -> Res<String> {
    let out = command(program, args).output()?;
    if !out.status.success() {
        return Err(format!("`{program}` failed ({})", out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Run a command, returning only whether it succeeded (output suppressed).
pub fn run_ok(program: &str, args: &[&str]) -> bool {
    command(program, args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// First `*.exe` (case-insensitive) in `dir`, sorted for determinism, or `None`
/// if the directory is missing or has none.
pub fn first_exe(dir: &Path) -> Res<Option<PathBuf>> {
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut exes: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
        })
        .collect();
    exes.sort();
    Ok(exes.into_iter().next())
}

/// Lowercase hex SHA-256 of a file's contents.
pub fn sha256_file(path: &Path) -> Res<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(to_hex_lower(&hasher.finalize()))
}

pub fn to_hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Verify a `SHA256SUMS` file inside `dir` (native, so it works on Windows
/// where `sha256sum -c` is unavailable). Each non-empty line is
/// `"<hex-hash>  <basename>"`. Returns an error listing any mismatch/missing.
pub fn verify_checksums(dir: &Path) -> Res<()> {
    let sums_path = dir.join("SHA256SUMS");
    let content = fs::read_to_string(&sums_path)
        .map_err(|e| format!("cannot read {}: {e}", sums_path.display()))?;

    let mut checked = 0usize;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // "<hash>  <name>" — split on the first run of whitespace.
        let (hash, name) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("malformed SHA256SUMS line: {line:?}"))?;
        let name = name.trim_start();
        let expected = hash.trim().to_lowercase();
        let file = dir.join(name);
        let actual = sha256_file(&file)
            .map_err(|e| format!("cannot hash {}: {e}", file.display()))?;
        if actual != expected {
            return Err(format!(
                "checksum mismatch for {name}: expected {expected}, got {actual}"
            )
            .into());
        }
        println!("  OK  {name}");
        checked += 1;
    }
    if checked == 0 {
        return Err("SHA256SUMS contained no entries".into());
    }
    Ok(())
}

/// Poll `http://127.0.0.1:8080/api/worlds` for an HTTP 200 for up to
/// `timeout_secs`, returning `true` on success.
pub fn poll_health(timeout_secs: u64) -> bool {
    println!("== Polling http://127.0.0.1:8080/api/worlds (up to {timeout_secs}s) ==");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(timeout_secs) {
        if http_ok() {
            println!("== Health check: HTTP 200 received ==");
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

/// One localhost health probe: connect, send a minimal GET, look for " 200"
/// in the status line. All errors collapse to `false`.
fn http_ok() -> bool {
    let addr: SocketAddr = match "127.0.0.1:8080".parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let req = "GET /api/worlds HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).contains(" 200")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(to_hex_lower(&[0x00, 0x0f, 0xab, 0xff]), "000fabff");
    }

    #[test]
    fn sha256_of_empty_is_known_vector() {
        let f = std::env::temp_dir().join("xtask_sha_empty.bin");
        fs::write(&f, b"").unwrap();
        let h = sha256_file(&f).unwrap();
        let _ = fs::remove_file(&f);
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn verify_checksums_accepts_matching_and_rejects_mismatch() {
        let dir = std::env::temp_dir().join("xtask_verify_test");
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("a.txt"), b"hello").unwrap();
        let good = sha256_file(&dir.join("a.txt")).unwrap();
        fs::write(dir.join("SHA256SUMS"), format!("{good}  a.txt\n")).unwrap();
        assert!(verify_checksums(&dir).is_ok());

        fs::write(dir.join("SHA256SUMS"), "deadbeef  a.txt\n").unwrap();
        assert!(verify_checksums(&dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_exe_none_for_missing_dir() {
        let missing = repo_root().join("definitely-not-a-real-dir-xyz");
        assert!(first_exe(&missing).unwrap().is_none());
    }
}

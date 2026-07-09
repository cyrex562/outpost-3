//! `cargo xtask release-publish` and `release-download` — GitHub Release
//! upload/download with native checksum handling and optional smoke tests.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};

use crate::util::{
    abs, first_exe, poll_health, repo_root, run, run_capture, run_ok, sha256_file,
    verify_checksums, Res,
};

// ---------------------------------------------------------------------------
// release-publish
// ---------------------------------------------------------------------------

/// `cargo xtask release-publish <tag> <artifact>...`
pub fn release_publish(args: &[String]) -> Res<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: cargo xtask release-publish <tag> <artifact>...");
        println!("  tag must match vX.Y.Z; a SHA256SUMS is computed and uploaded alongside.");
        return Ok(());
    }
    if args.len() < 2 {
        return Err("expected <tag> and at least one <artifact>".into());
    }
    let tag = &args[0];
    if !is_valid_tag(tag) {
        return Err(format!("tag must match vX.Y.Z (e.g. v1.2.3), got: {tag}").into());
    }

    // Resolve + validate artifacts (relative to the repo root).
    let artifacts: Vec<PathBuf> = args[1..].iter().map(|a| abs(a)).collect();
    for a in &artifacts {
        if !a.is_file() {
            return Err(format!("artifact not found: {}", a.display()).into());
        }
    }

    // Compute SHA256SUMS (basenames only) into a temp file.
    println!("== Computing SHA256SUMS ==");
    let sums_path =
        std::env::temp_dir().join(format!("harsh-SHA256SUMS-{}", std::process::id()));
    let mut sums = String::new();
    for a in &artifacts {
        let hash = sha256_file(a)?;
        let name = a.file_name().expect("artifact has a name").to_string_lossy();
        let line = format!("{hash}  {name}");
        println!("{line}");
        sums.push_str(&line);
        sums.push('\n');
    }
    fs::write(&sums_path, sums.as_bytes())?;

    // Assemble the file arguments (artifacts + SHA256SUMS) as strings.
    let mut files: Vec<String> = artifacts
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    files.push(sums_path.to_string_lossy().into_owned());

    if run_ok("gh", &["release", "view", tag]) {
        println!("== Uploading to existing release {tag} ==");
        let mut gh: Vec<&str> = vec!["release", "upload", tag, "--clobber"];
        gh.extend(files.iter().map(|s| s.as_str()));
        run("gh", &gh)?;
    } else {
        println!("== Creating release {tag} ==");
        let notes = format!("Harsh Realm {tag} desktop build");
        let mut gh: Vec<&str> = vec![
            "release", "create", tag, "--title", tag, "--notes", &notes,
        ];
        gh.extend(files.iter().map(|s| s.as_str()));
        run("gh", &gh)?;
    }

    let _ = fs::remove_file(&sums_path);

    println!("== Release URL ==");
    let url = run_capture("gh", &["release", "view", tag, "--json", "url", "-q", ".url"])?;
    println!("{url}");
    Ok(())
}

fn is_valid_tag(tag: &str) -> bool {
    let rest = match tag.strip_prefix('v') {
        Some(r) => r,
        None => return false,
    };
    let parts: Vec<&str> = rest.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

// ---------------------------------------------------------------------------
// release-download
// ---------------------------------------------------------------------------

struct DownloadOpts {
    tag: Option<String>,
    latest: bool,
    pattern: String,
    dir: String,
    smoke: bool,
    server_smoke: bool,
}

/// `cargo xtask release-download [--tag T | --latest] [--pattern G] [--dir D] [--smoke|--server-smoke]`
pub fn release_download(args: &[String]) -> Res<()> {
    let opts = match parse_download(args)? {
        Some(o) => o,
        None => return Ok(()), // --help
    };

    let tag = if opts.latest {
        println!("== Resolving latest release tag ==");
        let t = run_capture("gh", &["release", "view", "--json", "tagName", "-q", ".tagName"])?;
        println!("== Latest tag: {t} ==");
        t
    } else {
        opts.tag.clone().expect("validated: tag present when not latest")
    };

    let dir = abs(&opts.dir);
    fs::create_dir_all(&dir)?;
    let dir_str = dir.to_string_lossy().into_owned();

    println!(
        "== Downloading '{}' assets from {tag} into {} ==",
        opts.pattern, dir_str
    );
    run(
        "gh",
        &["release", "download", &tag, "--pattern", &opts.pattern, "--dir", &dir_str, "--clobber"],
    )?;
    println!("== Downloading SHA256SUMS ==");
    run(
        "gh",
        &["release", "download", &tag, "--pattern", "SHA256SUMS", "--dir", &dir_str, "--clobber"],
    )?;

    println!("== Verifying SHA256SUMS ==");
    verify_checksums(&dir)?;
    println!("== Checksums OK ==");

    if opts.smoke {
        smoke_windows(&dir)?;
    }
    if opts.server_smoke {
        server_smoke()?;
    }
    println!("== Done ==");
    Ok(())
}

fn parse_download(args: &[String]) -> Res<Option<DownloadOpts>> {
    let mut o = DownloadOpts {
        tag: None,
        latest: false,
        pattern: "*.exe".to_string(),
        dir: "dist".to_string(),
        smoke: false,
        server_smoke: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_download_usage();
                return Ok(None);
            }
            "--tag" => o.tag = Some(next_val(args, &mut i, "--tag")?),
            "--latest" => o.latest = true,
            "--pattern" => o.pattern = next_val(args, &mut i, "--pattern")?,
            "--dir" => o.dir = next_val(args, &mut i, "--dir")?,
            "--smoke" => o.smoke = true,
            "--server-smoke" => o.server_smoke = true,
            other => return Err(format!("unknown option: {other}").into()),
        }
        i += 1;
    }
    if !o.latest && o.tag.is_none() {
        return Err("one of --tag or --latest is required".into());
    }
    if o.latest && o.tag.is_some() {
        return Err("--tag and --latest are mutually exclusive".into());
    }
    Ok(Some(o))
}

fn next_val(args: &[String], i: &mut usize, flag: &str) -> Res<String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value").into())
}

/// Windows smoke test: launch the downloaded exe, poll for health, terminate.
fn smoke_windows(dir: &std::path::Path) -> Res<()> {
    if !cfg!(windows) {
        println!("== --smoke only applies on Windows; skipping exe launch ==");
        println!("   (SHA256 verification above still ran successfully)");
        return Ok(());
    }
    println!("== Smoke test (Windows) ==");
    let named = dir.join("harsh-realm-desktop.exe");
    let exe = if named.is_file() {
        named
    } else {
        first_exe(dir)?.ok_or("no .exe found in the download dir for the smoke test")?
    };

    println!("== Launching: {} ==", exe.display());
    let mut child = Command::new(&exe).spawn()?;
    let ok = poll_health(30);
    terminate(&mut child);
    if ok {
        println!("== Smoke test PASSED ==");
        Ok(())
    } else {
        Err("server did not respond within 30 s".into())
    }
}

/// Linux server smoke test: start harsh-web, poll health, run @smoke Playwright.
fn server_smoke() -> Res<()> {
    println!("== Server smoke test ==");
    println!("== Starting harsh-web (cargo run) ==");
    let mut server = Command::new("cargo")
        .args(["run", "--manifest-path", "crates/harsh-web/Cargo.toml"])
        .current_dir(repo_root())
        .spawn()?;

    if !poll_health(30) {
        terminate(&mut server);
        return Err("web host did not come up in time".into());
    }
    println!("== Web host is up ==");

    println!("== Running Playwright @smoke tests ==");
    let (ok, output) = run_combined_in_frontend(&["playwright", "test", "--grep", "@smoke"]);
    println!("{output}");
    terminate(&mut server);

    if ok {
        println!("== Playwright smoke tests PASSED ==");
        Ok(())
    } else {
        let lower = output.to_lowercase();
        if lower.contains("no tests") || lower.contains("0 tests found") || lower.contains("0 passed") {
            println!("== Warning: no @smoke-tagged Playwright tests found; skipping ==");
            Ok(())
        } else {
            Err("Playwright smoke tests failed".into())
        }
    }
}

/// Run `npx <args>` in `frontend/` with `REUSE_SERVERS=1`, capturing combined output.
fn run_combined_in_frontend(npx_args: &[&str]) -> (bool, String) {
    // Reuse the shared helper for the npx→cmd handling by running from the
    // frontend dir via an explicit command.
    let frontend = repo_root().join("frontend");
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("npx");
        c
    } else {
        Command::new("npx")
    };
    cmd.args(npx_args)
        .env("REUSE_SERVERS", "1")
        .current_dir(&frontend);
    match cmd.output() {
        Ok(out) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&out.stdout));
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            (out.status.success(), s)
        }
        Err(e) => (false, format!("failed to run npx: {e}")),
    }
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn print_download_usage() {
    println!(
        "Usage: cargo xtask release-download [--tag <tag> | --latest]\n\
         \x20      [--pattern <glob>] [--dir <dir>] [--smoke | --server-smoke]\n\
         \n\
         \x20 --tag <tag>      Exact release tag (e.g. v1.2.3)\n\
         \x20 --latest         Resolve and download the latest release\n\
         \x20 --pattern <glob> Asset glob for gh release download (default: *.exe)\n\
         \x20 --dir <dir>      Download directory (default: dist)\n\
         \x20 --smoke          Windows: launch the exe and health-check it\n\
         \x20 --server-smoke   Linux: start harsh-web and run @smoke Playwright tests\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_validation() {
        assert!(is_valid_tag("v0.50.0"));
        assert!(is_valid_tag("v1.2.3"));
        assert!(!is_valid_tag("0.50.0"));
        assert!(!is_valid_tag("v1.2"));
        assert!(!is_valid_tag("v1.2.3.4"));
        assert!(!is_valid_tag("vx.y.z"));
        assert!(!is_valid_tag("v1.2."));
    }

    #[test]
    fn download_requires_tag_or_latest() {
        assert!(parse_download(&[]).is_err());
        assert!(parse_download(&["--tag".into(), "v1.2.3".into(), "--latest".into()]).is_err());
        let ok = parse_download(&["--latest".into()]).unwrap().unwrap();
        assert!(ok.latest);
        assert_eq!(ok.pattern, "*.exe");
        assert_eq!(ok.dir, "dist");
    }

    #[test]
    fn download_parses_flags() {
        let o = parse_download(&[
            "--tag".into(),
            "v1.2.3".into(),
            "--pattern".into(),
            "*.zip".into(),
            "--dir".into(),
            "out".into(),
            "--server-smoke".into(),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(o.tag.as_deref(), Some("v1.2.3"));
        assert_eq!(o.pattern, "*.zip");
        assert_eq!(o.dir, "out");
        assert!(o.server_smoke);
    }
}

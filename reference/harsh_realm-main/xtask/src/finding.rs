//! `cargo xtask report-finding "<title>" ["<note>"]` — append a playtest/build
//! finding to `todo.md` with the next `HR-###` id.

use std::fs;

use crate::util::{repo_root, Res};

const SECTION_HEADER: &str = "## Playtest / build feedback";

pub fn report_finding(args: &[String]) -> Res<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }
    let title = args
        .first()
        .filter(|t| !t.is_empty())
        .ok_or("report-finding requires a <title>")?;
    let note = args.get(1).map(|s| s.as_str()).unwrap_or("");

    let root = repo_root();
    let todo_path = root.join("todo.md");
    let archive_path = root.join("todo-archive.md");
    let todo = fs::read_to_string(&todo_path)
        .map_err(|e| format!("cannot read {}: {e}", todo_path.display()))?;
    let archive = fs::read_to_string(&archive_path).unwrap_or_default();

    let next = next_hr_id(&[&todo, &archive]);
    let new_id = format!("HR-{next}");
    let item_line = format!("- [ ] {new_id}: {title}");

    let updated = insert_finding(&todo, &item_line, note);
    // Atomic-ish write: temp file in the same dir, then rename.
    let tmp = root.join("todo.md.xtask.tmp");
    fs::write(&tmp, updated.as_bytes())?;
    fs::rename(&tmp, &todo_path)?;

    println!("Added {new_id} to todo.md:");
    println!("  {item_line}");
    if !note.is_empty() {
        println!("    - {note}");
    }
    Ok(())
}

/// Highest `HR-<n>` across the given texts, plus one (starts at 1 if none).
fn next_hr_id(texts: &[&str]) -> u64 {
    let mut max = 0u64;
    for text in texts {
        let bytes = text.as_bytes();
        let mut i = 0;
        while let Some(pos) = text[i..].find("HR-") {
            let start = i + pos + 3;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                if let Ok(n) = text[start..j].parse::<u64>() {
                    max = max.max(n);
                }
            }
            i = j.max(i + pos + 3);
        }
    }
    max + 1
}

/// Append the finding under the feedback section. If the section exists, the
/// item lands at the end of it (before the next `## ` heading, else EOF). If
/// absent, the section is created at EOF so it stays a self-contained group and
/// does not sweep existing loose items under a new heading.
fn insert_finding(todo: &str, item_line: &str, note: &str) -> String {
    let note_line = |out: &mut Vec<String>| {
        if !note.is_empty() {
            out.push(format!("  - {note}"));
        }
    };

    let mut out: Vec<String> = Vec::new();

    if todo.lines().any(|l| l == SECTION_HEADER) {
        let mut in_section = false;
        let mut done = false;
        for line in todo.lines() {
            if line == SECTION_HEADER {
                in_section = true;
                out.push(line.to_string());
                continue;
            }
            if line.starts_with("## ") && in_section && !done {
                out.push(item_line.to_string());
                note_line(&mut out);
                done = true;
                in_section = false;
            }
            out.push(line.to_string());
        }
        if in_section && !done {
            out.push(item_line.to_string());
            note_line(&mut out);
        }
    } else {
        for line in todo.lines() {
            out.push(line.to_string());
        }
        out.push(String::new());
        out.push(SECTION_HEADER.to_string());
        out.push(String::new());
        out.push(item_line.to_string());
        note_line(&mut out);
    }

    let mut joined = out.join("\n");
    joined.push('\n');
    joined
}

fn print_usage() {
    println!(
        "Usage: cargo xtask report-finding \"<title>\" [\"<note>\"]\n\
         \n\
         Appends a finding to todo.md under \"{SECTION_HEADER}\" with the next HR-### id."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_is_max_plus_one_across_texts() {
        let a = "- [x] HR-100: done\n- [ ] HR-42: open";
        let b = "archive HR-768 and HR-7 here";
        assert_eq!(next_hr_id(&[a, b]), 769);
    }

    #[test]
    fn next_id_starts_at_one_when_none() {
        assert_eq!(next_hr_id(&["no ids here", ""]), 1);
    }

    #[test]
    fn creates_section_at_eof_without_disturbing_existing() {
        let todo = "# Title\n\n> note\n\n- [ ] HR-1: existing loose item\n";
        let out = insert_finding(todo, "- [ ] HR-2: new", "a note");
        // Existing content untouched at the top.
        assert!(out.starts_with("# Title\n\n> note\n\n- [ ] HR-1: existing loose item\n"));
        // New section + item appended at the end.
        assert!(out.contains("## Playtest / build feedback\n\n- [ ] HR-2: new\n  - a note\n"));
    }

    #[test]
    fn appends_into_existing_section_before_next_heading() {
        let todo = "## Playtest / build feedback\n\n- [ ] HR-1: first\n\n## Other\n- x\n";
        let out = insert_finding(todo, "- [ ] HR-2: second", "");
        // Second item is inside the feedback section, before "## Other".
        let fb = out.find("## Playtest / build feedback").unwrap();
        let other = out.find("## Other").unwrap();
        let second = out.find("HR-2: second").unwrap();
        assert!(fb < second && second < other, "new item must sit in the feedback section");
    }
}

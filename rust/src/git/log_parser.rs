use std::path::Path;
use std::process::Command;
use crate::types::Commit;

/// Runs `git log` and parses it into structured Commit objects.
/// Uses a "COMMIT|" prefix on each header line to distinguish it from
/// file-name lines in the mixed --name-only output.
pub fn parse_log(
    cwd: &Path,
    since: &str,
    path_filter: Option<&str>,
) -> Result<Vec<Commit>, String> {
    let mut args: Vec<String> = vec![
        "log".into(),
        "--format=COMMIT|%H|%ae|%ad|%s".into(),
        "--date=unix".into(),
        "--name-only".into(),
        "--diff-filter=ACDMRT".into(),
    ];

    if !since.is_empty() {
        args.push(format!("--since={since}"));
    }

    if let Some(p) = path_filter {
        args.push("--".into());
        args.push(p.into());
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed: {stderr}"));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_commit_output(&text))
}

fn parse_commit_output(output: &str) -> Vec<Commit> {
    let mut commits: Vec<Commit> = Vec::new();
    let mut current: Option<Commit> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("COMMIT|") {
            if let Some(c) = current.take() {
                commits.push(c);
            }
            // Format: hash|email|timestamp|subject  (subject may contain '|')
            let parts: Vec<&str> = rest.splitn(4, '|').collect();
            if parts.len() >= 4 {
                current = Some(Commit {
                    hash:      parts[0].to_string(),
                    author:    parts[1].to_string(),
                    timestamp: parts[2].parse().unwrap_or(0),
                    subject:   parts[3].to_string(),
                    files:     Vec::new(),
                });
            }
        } else if !trimmed.is_empty() {
            if let Some(ref mut c) = current {
                if let Some(file) = normalize_filename(trimmed) {
                    c.files.push(file);
                }
            }
        }
    }

    if let Some(c) = current {
        commits.push(c);
    }
    commits
}

/// Normalizes git rename notations:
///   "src/{old => new}/file.js" → "src/new/file.js"
///   "old-name => new-name"     → "new-name"
fn normalize_filename(raw: &str) -> Option<String> {
    if raw.contains('{') && raw.contains("=>") {
        // e.g. src/{old-dir => new-dir}/file.js
        let re = once_cell::sync::Lazy::force(&RENAME_RE);
        let result = re.replace(raw, "$1").replace("//", "/");
        return if result.contains('{') { None } else { Some(result.trim().to_string()) };
    }
    if raw.contains(" => ") {
        return raw.split(" => ").last().map(|s| s.trim().to_string());
    }
    Some(raw.to_string())
}

static RENAME_RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\{[^}]+ => ([^}]+)\}").unwrap());

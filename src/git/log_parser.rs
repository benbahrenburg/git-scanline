use crate::types::{Commit, DiffStats, DiffStatsMap};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

/// Runs a single `git log --numstat` and returns structured Commit objects
/// AND per-file line-level diff stats in one pass.
///
/// Previously two separate `git log` invocations were required (one `--name-only`,
/// one `--numstat`). Combining them into a single subprocess eliminates the
/// redundant git overhead.
pub fn parse_log(
    cwd: &Path,
    since: &str,
    path_filter: Option<&str>,
) -> Result<(Vec<Commit>, DiffStatsMap), String> {
    let mut args: Vec<String> = vec![
        "log".into(),
        "--format=COMMIT|%H|%ae|%ad|%s".into(),
        "--date=unix".into(),
        "--numstat".into(),
        "--diff-filter=ACDMRT".into(),
    ];

    if !since.is_empty() {
        args.push(format!("--since={since}"));
    }

    if let Some(p) = path_filter {
        args.push("--".into());
        args.push(p.into());
    }

    let mut child = Command::new("git")
        .args(&args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture git stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture git stderr".to_string())?;

    let stderr_reader = thread::spawn(move || {
        let mut stderr_text = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = reader.read_to_string(&mut stderr_text);
        stderr_text
    });

    let mut commits: Vec<Commit> = Vec::new();
    let mut diff_stats: DiffStatsMap = DiffStatsMap::new();
    let mut current: Option<Commit> = None;

    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|e| format!("Failed reading git output: {e}"))?;
        parse_commit_line(&line, &mut commits, &mut diff_stats, &mut current);
    }

    if let Some(c) = current.take() {
        commits.push(c);
    }

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for git process: {e}"))?;

    if !status.success() {
        let stderr_text = stderr_reader.join().unwrap_or_else(|_| String::new());
        return Err(format!("git log failed: {stderr_text}"));
    }

    let _ = stderr_reader.join();

    Ok((commits, diff_stats))
}

fn parse_commit_line(
    line: &str,
    commits: &mut Vec<Commit>,
    diff_stats: &mut DiffStatsMap,
    current: &mut Option<Commit>,
) {
    let trimmed = line.trim();

    if let Some(rest) = trimmed.strip_prefix("COMMIT|") {
        if let Some(c) = current.take() {
            commits.push(c);
        }
        let mut parts = rest.splitn(4, '|');
        if let (Some(hash), Some(author), Some(timestamp), Some(subject)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            *current = Some(Commit {
                hash: hash.to_string(),
                author: author.to_string(),
                timestamp: timestamp.parse().unwrap_or(0),
                subject: subject.to_string(),
                files: Vec::new(),
            });
        }
    } else if trimmed.is_empty() {
        // blank lines between commits — ignored
    } else {
        let mut parts = trimmed.splitn(3, '\t');
        if let (Some(added_raw), Some(deleted_raw), Some(raw_name)) =
            (parts.next(), parts.next(), parts.next())
        {
            if let Some(filename) = normalize_filename(raw_name) {
                if added_raw != "-" && deleted_raw != "-" {
                    let additions: usize = added_raw.parse().unwrap_or(0);
                    let deletions: usize = deleted_raw.parse().unwrap_or(0);
                    let entry: &mut DiffStats = diff_stats.entry(filename.clone()).or_default();
                    entry.additions += additions;
                    entry.deletions += deletions;
                }
                if let Some(ref mut c) = current {
                    c.files.push(filename);
                }
            }
        }
    }
}

/// Normalizes git rename notations:
///   "src/{old => new}/file.js" → "src/new/file.js"
///   "old-name => new-name"     → "new-name"
fn normalize_filename(raw: &str) -> Option<String> {
    if raw.contains('{') && raw.contains("=>") {
        let re = once_cell::sync::Lazy::force(&RENAME_RE);
        let result = re.replace(raw, "$1").replace("//", "/");
        return if result.contains('{') {
            None
        } else {
            Some(result.trim().to_string())
        };
    }
    if raw.contains(" => ") {
        return raw.split(" => ").last().map(|s| s.trim().to_string());
    }
    let t = raw.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

static RENAME_RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\{[^}]+ => ([^}]+)\}").unwrap());

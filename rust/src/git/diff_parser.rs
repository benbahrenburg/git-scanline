use std::path::Path;
use std::process::Command;
use crate::types::DiffStatsMap;

/// Runs `git log --numstat` and accumulates line-level churn per file.
pub fn parse_diff(
    cwd: &Path,
    since: &str,
    path_filter: Option<&str>,
) -> Result<DiffStatsMap, String> {
    let mut args: Vec<String> = vec![
        "log".into(),
        "--format=COMMIT|%H".into(),
        "--numstat".into(),
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
        .map_err(|e| format!("Failed to run git numstat: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git numstat failed: {stderr}"));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_numstat_output(&text))
}

fn parse_numstat_output(output: &str) -> DiffStatsMap {
    let mut stats: DiffStatsMap = DiffStatsMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("COMMIT|") {
            continue;
        }

        // numstat format: <additions>\t<deletions>\t<filename>
        let parts: Vec<&str> = trimmed.splitn(3, '\t').collect();
        if parts.len() < 3 { continue; }

        // Binary files show '-'
        if parts[0] == "-" || parts[1] == "-" { continue; }

        let additions: usize = parts[0].parse().unwrap_or(0);
        let deletions: usize = parts[1].parse().unwrap_or(0);

        let filename = normalize_numstat_filename(parts[2]);
        if let Some(fname) = filename {
            let entry = stats.entry(fname).or_default();
            entry.additions += additions;
            entry.deletions += deletions;
        }
    }

    stats
}

fn normalize_numstat_filename(raw: &str) -> Option<String> {
    if raw.contains('{') && raw.contains("=>") {
        let re = once_cell::sync::Lazy::force(&RENAME_RE);
        let result = re.replace(raw, "$1").replace("//", "/");
        return if result.contains('{') { None } else { Some(result.trim().to_string()) };
    }
    if raw.contains(" => ") {
        return raw.split(" => ").last().map(|s| s.trim().to_string());
    }
    let t = raw.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

static RENAME_RE: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\{[^}]+ => ([^}]+)\}").unwrap());

use crate::types::{Commit, SecurityRisk};
use once_cell::sync::Lazy;
use regex::Regex;

static ENV_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:^|/)\.(env)(\.|$)").expect("env regex"));
static KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\.(pem|key|p12|pfx|cer|crt|jks|ppk|keystore)$").expect("key regex")
});
static CRED_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|/)(?:credential|secret|password|passwd|private[_-]?key|api[_-]?key|auth[_-]?token)[^/]*$").expect("cred regex")
});

fn get_risk_type(file: &str) -> Option<&'static str> {
    if ENV_PATTERN.is_match(file) {
        return Some("env-file");
    }
    if KEY_PATTERN.is_match(file) {
        return Some("key-or-cert");
    }
    if CRED_PATTERN.is_match(file) {
        return Some("credential-file");
    }
    None
}

/// Scans raw (unfiltered) commits for security-sensitive files ever committed
/// to git history. Even deleted files are flagged — they remain accessible.
pub fn analyze_security(commits: &[Commit]) -> Vec<SecurityRisk> {
    use std::collections::HashMap;

    struct Entry {
        risk_type: &'static str,
        count: usize,
        first: i64,
        last: i64,
    }

    let mut risks: HashMap<String, Entry> = HashMap::new();

    for commit in commits {
        for file in &commit.files {
            let Some(risk_type) = get_risk_type(file) else {
                continue;
            };
            risks
                .entry(file.clone())
                .and_modify(|e| {
                    e.count += 1;
                    if commit.timestamp < e.first {
                        e.first = commit.timestamp;
                    }
                    if commit.timestamp > e.last {
                        e.last = commit.timestamp;
                    }
                })
                .or_insert(Entry {
                    risk_type,
                    count: 1,
                    first: commit.timestamp,
                    last: commit.timestamp,
                });
        }
    }

    let mut out: Vec<SecurityRisk> = risks
        .into_iter()
        .map(|(file, e)| SecurityRisk {
            file,
            risk_type: e.risk_type.to_string(),
            commit_count: e.count,
            first_seen: fmt_date(e.first),
            last_seen: fmt_date(e.last),
        })
        .collect();

    out.sort_by(|a, b| b.commit_count.cmp(&a.commit_count));
    out
}

fn fmt_date(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

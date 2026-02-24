use std::collections::HashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::types::{Commit, CommitQualityData};

static WIP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(wip|temp|tmp|fixup|squash|hack|dirty|oops|typo|debug|draft)\b|^(fix|update|changes|stuff|misc|test|cleanup|commit|save|ok|done)[.!\s]*$")
        .expect("WIP regex")
});

const LARGE_COMMIT_THRESHOLD: usize = 30;
const SHORT_MSG_MIN_LENGTH:    usize = 10;

/// Tracks per-file involvement in low-quality commits (WIP/short messages)
/// and oversized commits (mass reformats, merge-all).
pub fn analyze_commit_quality(
    commits: &[Commit],
    files: &[String],
) -> HashMap<String, CommitQualityData> {
    let file_set: std::collections::HashSet<&str> =
        files.iter().map(|s| s.as_str()).collect();

    let mut wip_counts:   HashMap<String, usize> = HashMap::new();
    let mut large_counts: HashMap<String, usize> = HashMap::new();

    for commit in commits {
        let subj = commit.subject.trim();
        let is_wip   = WIP_PATTERN.is_match(subj) || subj.len() < SHORT_MSG_MIN_LENGTH;
        let is_large = commit.files.len() > LARGE_COMMIT_THRESHOLD;

        for file in &commit.files {
            if !file_set.contains(file.as_str()) { continue; }
            if is_wip   { *wip_counts.entry(file.clone()).or_default()   += 1; }
            if is_large { *large_counts.entry(file.clone()).or_default() += 1; }
        }
    }

    let max_wip   = wip_counts.values().copied().max().unwrap_or(1).max(1) as f64;
    let max_large = large_counts.values().copied().max().unwrap_or(1).max(1) as f64;

    files.iter().map(|file| {
        let wip   = *wip_counts.get(file).unwrap_or(&0);
        let large = *large_counts.get(file).unwrap_or(&0);
        let score = (wip as f64 / max_wip) * 60.0 + (large as f64 / max_large) * 40.0;
        (file.clone(), CommitQualityData {
            wip_commits:        wip,
            large_commit_count: large,
            commit_quality_score: score,
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Commit;

    fn make_commit(subject: &str, files: &[&str]) -> Commit {
        Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp: 1700000000,
            subject: subject.to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_detects_wip_keyword() {
        let commits = vec![
            make_commit("WIP: still working on this", &["a.rs"]),
            make_commit("implement user authentication properly", &["b.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let result = analyze_commit_quality(&commits, &files);
        assert!(result["a.rs"].wip_commits > 0, "WIP commit should be detected for a.rs");
        assert_eq!(result["b.rs"].wip_commits, 0, "Well-described commit should not count as WIP");
    }

    #[test]
    fn test_detects_short_message_as_wip() {
        // Messages under 10 chars count as low quality
        let commits = vec![make_commit("fix", &["a.rs"])];
        let files = vec!["a.rs".to_string()];
        let result = analyze_commit_quality(&commits, &files);
        assert!(result["a.rs"].wip_commits > 0, "Short commit message should count as WIP");
    }

    #[test]
    fn test_detects_large_commit() {
        // Commits touching > 30 files count as large
        let large_files: Vec<String> = (0..35).map(|i| format!("src/file{i}.rs")).collect();
        let commit = Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp: 1700000000,
            subject: "massive reformat of entire codebase".to_string(),
            files: large_files.clone(),
        };
        let tracked = vec!["src/file0.rs".to_string()];
        let result = analyze_commit_quality(&[commit], &tracked);
        assert!(result["src/file0.rs"].large_commit_count > 0, "Large commit should be detected");
    }

    #[test]
    fn test_quality_score_in_range() {
        let commits = vec![
            make_commit("wip", &["a.rs"]),
            make_commit("tmp hack", &["a.rs"]),
        ];
        let files = vec!["a.rs".to_string()];
        let result = analyze_commit_quality(&commits, &files);
        let score = result["a.rs"].commit_quality_score;
        assert!(score >= 0.0 && score <= 100.0, "commit_quality_score {} out of range", score);
    }

    #[test]
    fn test_clean_commits_score_zero() {
        let commits = vec![
            make_commit("implement OAuth2 token refresh with retry logic", &["auth.rs"]),
            make_commit("add unit tests for token expiry edge cases", &["auth.rs"]),
        ];
        let files = vec!["auth.rs".to_string()];
        let result = analyze_commit_quality(&commits, &files);
        assert_eq!(result["auth.rs"].wip_commits, 0, "Well-described commits should not be WIP");
        assert_eq!(result["auth.rs"].large_commit_count, 0, "Small commits should not be large");
    }
}

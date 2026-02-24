use std::collections::{HashMap, HashSet};
use crate::types::{Commit, BugData};

static BUG_PATTERN: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?i)\b(fix|bug|patch|hotfix|regression|broken|crash|defect|issue|error)\b").unwrap()
});

/// Identifies files that frequently appear in bug-fix commits.
pub fn analyze_bug_correlation(commits: &[Commit], files: &[String]) -> HashMap<String, BugData> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    let mut file_bug_counts: HashMap<String, usize> = HashMap::new();

    for commit in commits {
        if !BUG_PATTERN.is_match(&commit.subject) { continue; }
        for file in &commit.files {
            if !file_set.contains(file.as_str()) { continue; }
            *file_bug_counts.entry(file.clone()).or_insert(0) += 1;
        }
    }

    let max_count = file_bug_counts.values().cloned().fold(0.0001_f64, |a, b| a.max(b as f64));

    files.iter().map(|file| {
        let count = file_bug_counts.get(file).cloned().unwrap_or(0);
        (file.clone(), BugData {
            bug_commits: count,
            bug_score:   (count as f64 / max_count) * 100.0,
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
    fn test_detects_fix_commits() {
        let commits = vec![
            make_commit("fix: null pointer in auth", &["src/auth.rs"]),
            make_commit("add feature", &["src/feature.rs"]),
        ];
        let files = vec!["src/auth.rs".to_string(), "src/feature.rs".to_string()];
        let result = analyze_bug_correlation(&commits, &files);
        assert!(result["src/auth.rs"].bug_commits > 0, "auth.rs should have bug commits");
        assert_eq!(result["src/feature.rs"].bug_commits, 0, "feature.rs should have no bug commits");
    }

    #[test]
    fn test_detects_multiple_bug_keywords() {
        for kw in &["fix", "bug", "patch", "hotfix", "regression", "crash", "defect", "error"] {
            let commits = vec![make_commit(&format!("{} something", kw), &["src/a.rs"])];
            let files = vec!["src/a.rs".to_string()];
            let result = analyze_bug_correlation(&commits, &files);
            assert!(
                result["src/a.rs"].bug_commits > 0,
                "keyword '{}' should trigger bug detection", kw
            );
        }
    }

    #[test]
    fn test_scores_in_range() {
        let commits = vec![
            make_commit("fix crash", &["a.rs", "b.rs"]),
            make_commit("fix more", &["a.rs"]),
            make_commit("regular commit", &["b.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let result = analyze_bug_correlation(&commits, &files);
        for (_, data) in &result {
            assert!(data.bug_score >= 0.0 && data.bug_score <= 100.0,
                "bug_score {} out of range", data.bug_score);
        }
    }

    #[test]
    fn test_highest_bug_count_gets_score_100() {
        let commits = vec![
            make_commit("fix a", &["hot.rs"]),
            make_commit("fix b", &["hot.rs"]),
            make_commit("fix c", &["cold.rs"]),
        ];
        let files = vec!["hot.rs".to_string(), "cold.rs".to_string()];
        let result = analyze_bug_correlation(&commits, &files);
        // hot.rs has 2 bug commits, cold.rs has 1 — hot.rs should score 100
        assert!((result["hot.rs"].bug_score - 100.0).abs() < 0.001,
            "Most-blamed file should score 100");
        assert!(result["cold.rs"].bug_score < 100.0);
    }
}

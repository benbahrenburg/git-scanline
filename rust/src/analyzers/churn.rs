use std::collections::{HashMap, HashSet};
use crate::types::{Commit, ChurnData};

// Exponential decay: λ = 0.005 → half-life ≈ 139 days
const DECAY_LAMBDA: f64 = 0.005;

/// Calculates churn rate with recency decay per file.
/// Recent commits contribute exponentially more to the weighted score.
pub fn analyze_churn(commits: &[Commit], files: &[String]) -> HashMap<String, ChurnData> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();
    let now = chrono::Utc::now().timestamp();

    // filename → (commit_count, weighted_churn)
    let mut file_churn: HashMap<String, (usize, f64)> = HashMap::new();

    for commit in commits {
        let days_ago = ((now - commit.timestamp) / 86400).max(0) as f64;
        let decay_weight = (-DECAY_LAMBDA * days_ago).exp();

        for file in &commit.files {
            if !file_set.contains(file.as_str()) { continue; }
            let entry = file_churn.entry(file.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += decay_weight;
        }
    }

    let total_commits = commits.len().max(1) as f64;
    let max_weighted = file_churn.values().map(|(_, w)| *w).fold(0.0001_f64, f64::max);

    files.iter().map(|file| {
        let (count, weighted) = file_churn.get(file).cloned().unwrap_or((0, 0.0));
        let data = ChurnData {
            commit_count:   count,
            raw_score:      ((count as f64 / total_commits) * 500.0).min(100.0),
            weighted_score: (weighted / max_weighted) * 100.0,
        };
        (file.clone(), data)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Commit;

    fn make_commit(files: &[&str]) -> Commit {
        Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            subject: "change".to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_churn_scores_in_range() {
        let commits = vec![
            make_commit(&["src/a.rs", "src/b.rs"]),
            make_commit(&["src/a.rs"]),
            make_commit(&["src/a.rs", "src/c.rs"]),
        ];
        let files = vec!["src/a.rs".to_string(), "src/b.rs".to_string(), "src/c.rs".to_string()];
        let result = analyze_churn(&commits, &files);
        for (_, data) in &result {
            assert!(data.weighted_score >= 0.0 && data.weighted_score <= 100.0,
                "weighted_score {} out of range", data.weighted_score);
            assert!(data.raw_score >= 0.0 && data.raw_score <= 100.0,
                "raw_score {} out of range", data.raw_score);
        }
    }

    #[test]
    fn test_most_churned_file_has_highest_score() {
        let commits = vec![
            make_commit(&["hot.rs", "cold.rs"]),
            make_commit(&["hot.rs"]),
            make_commit(&["hot.rs"]),
        ];
        let files = vec!["hot.rs".to_string(), "cold.rs".to_string()];
        let result = analyze_churn(&commits, &files);
        assert!(
            result["hot.rs"].weighted_score > result["cold.rs"].weighted_score,
            "hot.rs appears in more commits and should have a higher score"
        );
    }

    #[test]
    fn test_file_not_in_commits_has_zero_count() {
        let commits = vec![make_commit(&["a.rs"])];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let result = analyze_churn(&commits, &files);
        assert_eq!(result["b.rs"].commit_count, 0, "b.rs was never committed, count must be 0");
        assert_eq!(result["b.rs"].weighted_score, 0.0, "b.rs was never committed, score must be 0");
    }

    #[test]
    fn test_all_files_present_in_output() {
        let commits = vec![make_commit(&["a.rs"])];
        let files = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        let result = analyze_churn(&commits, &files);
        assert_eq!(result.len(), 3, "Output should contain every input file");
    }
}

use crate::types::{BurstData, Commit};
use std::collections::{HashMap, HashSet};

const BURST_WINDOW_SECS: i64 = 24 * 3600;
const BURST_MIN_COMMITS: usize = 3;

/// Detects rapid successive commit bursts (patch-on-patch behavior) per file.
pub fn analyze_bursts(commits: &[Commit], files: &[String]) -> HashMap<String, BurstData> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    // filename → sorted list of timestamps
    let mut file_timestamps: HashMap<String, Vec<i64>> = HashMap::new();

    for commit in commits {
        for file in &commit.files {
            if !file_set.contains(file.as_str()) {
                continue;
            }
            file_timestamps
                .entry(file.clone())
                .or_default()
                .push(commit.timestamp);
        }
    }

    let mut raw: HashMap<String, usize> = HashMap::new();

    for file in files {
        let mut timestamps = file_timestamps.get(file).cloned().unwrap_or_default();
        timestamps.sort_unstable();

        let mut burst_incidents = 0usize;
        let mut i = 0;
        while i < timestamps.len() {
            let mut count = 1;
            let mut j = i + 1;
            while j < timestamps.len() && timestamps[j] - timestamps[i] <= BURST_WINDOW_SECS {
                count += 1;
                j += 1;
            }
            if count >= BURST_MIN_COMMITS {
                burst_incidents += 1;
                i = j;
            } else {
                i += 1;
            }
        }
        raw.insert(file.clone(), burst_incidents);
    }

    let max_bursts = raw
        .values()
        .cloned()
        .fold(0.0001_f64, |a, b| a.max(b as f64));

    files
        .iter()
        .map(|file| {
            let incidents = *raw.get(file).unwrap_or(&0);
            (
                file.clone(),
                BurstData {
                    burst_incidents: incidents,
                    burst_score: (incidents as f64 / max_bursts) * 100.0,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Commit;

    fn commit_at(timestamp: i64, files: &[&str]) -> Commit {
        Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp,
            subject: "change".to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_detects_burst_within_24h() {
        // 3 commits within 1 hour on hot.rs — should count as a burst
        let base = 1700000000i64;
        let commits = vec![
            commit_at(base, &["hot.rs"]),
            commit_at(base + 1_800, &["hot.rs"]), // +30 min
            commit_at(base + 3_600, &["hot.rs"]), // +1 hr
            commit_at(base + 86_400 * 10, &["cold.rs"]), // 10 days later, different file
        ];
        let files = vec!["hot.rs".to_string(), "cold.rs".to_string()];
        let result = analyze_bursts(&commits, &files);
        assert!(
            result["hot.rs"].burst_incidents > 0,
            "hot.rs should have a burst incident"
        );
        assert_eq!(
            result["cold.rs"].burst_incidents, 0,
            "cold.rs has only one commit, no burst"
        );
    }

    #[test]
    fn test_no_burst_for_sparse_commits() {
        let base = 1700000000i64;
        let commits = vec![
            commit_at(base, &["a.rs"]),
            commit_at(base + 86_400 * 5, &["a.rs"]), // 5 days apart
            commit_at(base + 86_400 * 10, &["a.rs"]), // 5 days apart
        ];
        let files = vec!["a.rs".to_string()];
        let result = analyze_bursts(&commits, &files);
        assert_eq!(
            result["a.rs"].burst_incidents, 0,
            "Sparse commits should not register as a burst"
        );
    }

    #[test]
    fn test_burst_requires_at_least_3_commits() {
        // Only 2 commits within 24h — below the burst threshold
        let base = 1700000000i64;
        let commits = vec![
            commit_at(base, &["a.rs"]),
            commit_at(base + 3_600, &["a.rs"]),
        ];
        let files = vec!["a.rs".to_string()];
        let result = analyze_bursts(&commits, &files);
        assert_eq!(
            result["a.rs"].burst_incidents, 0,
            "Two commits within 24h should not count as a burst"
        );
    }

    #[test]
    fn test_burst_score_in_range() {
        let base = 1700000000i64;
        let commits = vec![
            commit_at(base, &["a.rs"]),
            commit_at(base + 100, &["a.rs"]),
            commit_at(base + 200, &["a.rs"]),
        ];
        let files = vec!["a.rs".to_string()];
        let result = analyze_bursts(&commits, &files);
        let score = result["a.rs"].burst_score;
        assert!(
            score >= 0.0 && score <= 100.0,
            "burst_score {} out of range",
            score
        );
    }
}

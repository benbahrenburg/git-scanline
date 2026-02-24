use crate::types::{Commit, CouplingEntry};
use std::collections::{HashMap, HashSet};

// Skip commits touching more than this many files (large merges/reformats)
const MAX_FILES_PER_COMMIT: usize = 20;

/// Builds a co-change coupling matrix using Jaccard similarity.
/// Files that always change together suggest hidden dependencies.
pub fn analyze_coupling(commits: &[Commit], files: &[String]) -> Vec<CouplingEntry> {
    let file_set: HashSet<&str> = files.iter().map(|s| s.as_str()).collect();

    let mut pair_counts: HashMap<String, usize> = HashMap::new();
    let mut file_counts: HashMap<String, usize> = HashMap::new();

    for commit in commits {
        let touched: Vec<&str> = commit
            .files
            .iter()
            .filter(|f| file_set.contains(f.as_str()))
            .map(|s| s.as_str())
            .collect();

        for &file in &touched {
            *file_counts.entry(file.to_string()).or_insert(0) += 1;
        }

        if touched.len() > MAX_FILES_PER_COMMIT {
            continue;
        }

        for i in 0..touched.len() {
            for j in (i + 1)..touched.len() {
                let mut pair = [touched[i], touched[j]];
                pair.sort_unstable();
                let key = format!("{}||{}", pair[0], pair[1]);
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let mut couplings: Vec<CouplingEntry> = pair_counts
        .iter()
        .filter(|(_, &count)| count >= 3)
        .filter_map(|(key, &co_changes)| {
            let mut parts = key.splitn(2, "||");
            let file_a = parts.next()?.to_string();
            let file_b = parts.next()?.to_string();

            let total_a = *file_counts.get(&file_a).unwrap_or(&1);
            let total_b = *file_counts.get(&file_b).unwrap_or(&1);
            let union = total_a + total_b - co_changes;
            let strength = if union > 0 {
                (co_changes as f64 / union as f64) * 100.0
            } else {
                0.0
            };

            Some(CouplingEntry {
                file_a,
                file_b,
                co_changes,
                strength,
            })
        })
        .collect();

    couplings.sort_by(|a, b| b.co_changes.cmp(&a.co_changes));
    couplings
}

/// Returns a per-file coupling score: each file's maximum Jaccard strength.
pub fn get_coupling_scores(files: &[String], couplings: &[CouplingEntry]) -> HashMap<String, f64> {
    let mut scores: HashMap<String, f64> = files.iter().map(|f| (f.clone(), 0.0)).collect();
    for c in couplings {
        scores
            .entry(c.file_a.clone())
            .and_modify(|s| *s = s.max(c.strength));
        scores
            .entry(c.file_b.clone())
            .and_modify(|s| *s = s.max(c.strength));
    }
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Commit;

    fn make_commit(files: &[&str]) -> Commit {
        Commit {
            hash: "abc".to_string(),
            author: "dev@test.com".to_string(),
            timestamp: 1700000000,
            subject: "change".to_string(),
            files: files.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_detects_coupled_files() {
        // a.rs and b.rs change together 3 times — should be coupled
        let commits = vec![
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["c.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        let result = analyze_coupling(&commits, &files);
        assert!(!result.is_empty(), "Should detect a coupled pair");
        let pair = result.iter().find(|e| {
            (e.file_a == "a.rs" && e.file_b == "b.rs") || (e.file_a == "b.rs" && e.file_b == "a.rs")
        });
        assert!(
            pair.is_some(),
            "a.rs and b.rs should be identified as coupled"
        );
    }

    #[test]
    fn test_no_coupling_below_threshold() {
        // Files only co-change twice — below the minimum of 3
        let commits = vec![
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let result = analyze_coupling(&commits, &files);
        assert!(
            result.is_empty(),
            "Two co-changes should not meet the coupling threshold"
        );
    }

    #[test]
    fn test_coupling_scores_populated_for_coupled_pair() {
        let commits = vec![
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
        ];
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let couplings = analyze_coupling(&commits, &files);
        let scores = get_coupling_scores(&files, &couplings);
        assert!(
            scores["a.rs"] > 0.0,
            "a.rs should have a non-zero coupling score"
        );
        assert!(
            scores["b.rs"] > 0.0,
            "b.rs should have a non-zero coupling score"
        );
    }

    #[test]
    fn test_uncoupled_file_scores_zero() {
        let commits = vec![
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["a.rs", "b.rs"]),
            make_commit(&["lone.rs"]),
        ];
        let files = vec![
            "a.rs".to_string(),
            "b.rs".to_string(),
            "lone.rs".to_string(),
        ];
        let couplings = analyze_coupling(&commits, &files);
        let scores = get_coupling_scores(&files, &couplings);
        assert_eq!(
            scores["lone.rs"], 0.0,
            "lone.rs never co-changes, should score 0"
        );
    }

    #[test]
    fn test_large_commits_are_excluded_from_coupling() {
        // The threshold is based on how many *watched* files appear in a single commit.
        // A commit where > 20 watched files all change together is excluded (mass reformat).
        // Build 21 watched files that all co-change 3 times — pairing should be skipped.
        let watched: Vec<String> = (0..21).map(|i| format!("watched{i}.rs")).collect();
        let watched_refs: Vec<&str> = watched.iter().map(|s| s.as_str()).collect();
        let commits: Vec<Commit> = (0..3).map(|_| make_commit(&watched_refs)).collect();
        let result = analyze_coupling(&commits, &watched);
        assert!(
            result.is_empty(),
            "Commits with > 20 watched files should be excluded from coupling analysis"
        );
    }
}

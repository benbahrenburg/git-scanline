use std::collections::HashMap;
use crate::analyzers::coupling::get_coupling_scores;
use crate::types::*;

const TIER_CRITICAL: f64 = 75.0;
const TIER_HIGH:     f64 = 50.0;
const TIER_MEDIUM:   f64 = 25.0;

/// Aggregates all analyzer outputs into a final Hotspot Score (0–100) per file.
#[allow(clippy::too_many_arguments)]
pub fn score_hotspots(
    files:               &[String],
    churn_data:          &HashMap<String, ChurnData>,
    bug_data:            &HashMap<String, BugData>,
    revert_data:         &HashMap<String, RevertData>,
    burst_data:          &HashMap<String, BurstData>,
    coupling_data:       &[CouplingEntry],
    silo_data:           &HashMap<String, SiloData>,
    commit_quality_data: &HashMap<String, CommitQualityData>,
    diff_stats:          &DiffStatsMap,
    weights:             &Weights,
) -> Vec<HotspotResult> {
    let coupling_scores = get_coupling_scores(files, coupling_data);

    files.iter().map(|file| {
        let churn   = churn_data.get(file);
        let bugs    = bug_data.get(file);
        let reverts = revert_data.get(file);
        let bursts  = burst_data.get(file);
        let silo    = silo_data.get(file);
        let cq      = commit_quality_data.get(file);
        let diff    = diff_stats.get(file);

        let churn_score          = churn.map_or(0.0, |d| d.weighted_score);
        let bug_fix_score        = bugs.map_or(0.0, |d| d.bug_score);
        let revert_score         = reverts.map_or(0.0, |d| d.revert_score);
        let burst_score          = bursts.map_or(0.0, |d| d.burst_score);
        let coupling_score       = *coupling_scores.get(file).unwrap_or(&0.0);
        let silo_score           = silo.map_or(0.0, |d| d.top_author_percent);
        let commit_quality_score = cq.map_or(0.0, |d| d.commit_quality_score);

        let hotspot_score =
            churn_score          * weights.churn          +
            bug_fix_score        * weights.bugs           +
            revert_score         * weights.reverts        +
            burst_score          * weights.bursts         +
            coupling_score       * weights.coupling       +
            silo_score           * weights.silo           +
            commit_quality_score * weights.commit_quality;

        HotspotResult {
            file: file.clone(),
            hotspot_score,
            churn_score,
            bug_fix_score,
            revert_score,
            burst_score,
            coupling_score,
            silo_score,
            commit_quality_score,
            tier: get_tier(hotspot_score),
            details: HotspotDetails {
                commit_count:       churn.map_or(0, |d| d.commit_count),
                bug_commits:        bugs.map_or(0, |d| d.bug_commits),
                revert_count:       reverts.map_or(0, |d| d.revert_count),
                burst_incidents:    bursts.map_or(0, |d| d.burst_incidents),
                wip_commits:        cq.map_or(0, |d| d.wip_commits),
                large_commit_count: cq.map_or(0, |d| d.large_commit_count),
                top_author:         silo.map_or_else(|| "unknown".to_string(), |d| d.top_author.clone()),
                top_author_percent: silo.map_or(0.0, |d| d.top_author_percent),
                author_count:       silo.map_or(1, |d| d.author_count),
                additions:          diff.map_or(0, |d| d.additions),
                deletions:          diff.map_or(0, |d| d.deletions),
            },
        }
    }).collect()
}

fn get_tier(score: f64) -> Tier {
    if score >= TIER_CRITICAL { Tier::Critical }
    else if score >= TIER_HIGH { Tier::High }
    else if score >= TIER_MEDIUM { Tier::Medium }
    else { Tier::Low }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_churn(files: &[String]) -> HashMap<String, ChurnData> {
        files.iter().map(|f| (f.clone(), ChurnData {
            commit_count: 0, weighted_score: 0.0, raw_score: 0.0,
        })).collect()
    }

    fn zero_bugs(files: &[String]) -> HashMap<String, BugData> {
        files.iter().map(|f| (f.clone(), BugData { bug_commits: 0, bug_score: 0.0 })).collect()
    }

    fn zero_reverts(files: &[String]) -> HashMap<String, RevertData> {
        files.iter().map(|f| (f.clone(), RevertData { revert_count: 0, revert_score: 0.0 })).collect()
    }

    fn zero_bursts(files: &[String]) -> HashMap<String, BurstData> {
        files.iter().map(|f| (f.clone(), BurstData { burst_incidents: 0, burst_score: 0.0 })).collect()
    }

    fn zero_silo(files: &[String]) -> HashMap<String, SiloData> {
        files.iter().map(|f| (f.clone(), SiloData {
            top_author: "dev".to_string(), top_author_percent: 0.0, author_count: 1,
        })).collect()
    }

    fn zero_quality(files: &[String]) -> HashMap<String, CommitQualityData> {
        files.iter().map(|f| (f.clone(), CommitQualityData {
            wip_commits: 0, large_commit_count: 0, commit_quality_score: 0.0,
        })).collect()
    }

    #[test]
    fn test_all_zero_signals_gives_zero_score() {
        let files = vec!["src/main.rs".to_string()];
        let results = score_hotspots(
            &files,
            &zero_churn(&files), &zero_bugs(&files), &zero_reverts(&files),
            &zero_bursts(&files), &[], &zero_silo(&files), &zero_quality(&files),
            &DiffStatsMap::new(), &Weights::default(),
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].hotspot_score, 0.0, "All-zero signals should produce score 0");
        assert_eq!(results[0].tier, Tier::Low, "Score 0 should be Tier::Low");
    }

    #[test]
    fn test_tier_thresholds() {
        assert_eq!(get_tier(100.0), Tier::Critical, "100 should be Critical");
        assert_eq!(get_tier(75.0),  Tier::Critical, "75 should be Critical");
        assert_eq!(get_tier(74.9),  Tier::High,     "74.9 should be High");
        assert_eq!(get_tier(50.0),  Tier::High,     "50 should be High");
        assert_eq!(get_tier(49.9),  Tier::Medium,   "49.9 should be Medium");
        assert_eq!(get_tier(25.0),  Tier::Medium,   "25 should be Medium");
        assert_eq!(get_tier(24.9),  Tier::Low,      "24.9 should be Low");
        assert_eq!(get_tier(0.0),   Tier::Low,      "0 should be Low");
    }

    #[test]
    fn test_churn_weight_applied_correctly() {
        let files = vec!["a.rs".to_string()];
        let mut churn = zero_churn(&files);
        churn.get_mut("a.rs").unwrap().weighted_score = 100.0;
        let results = score_hotspots(
            &files, &churn, &zero_bugs(&files), &zero_reverts(&files),
            &zero_bursts(&files), &[], &zero_silo(&files), &zero_quality(&files),
            &DiffStatsMap::new(), &Weights::default(),
        );
        let expected = 100.0 * Weights::default().churn;
        let actual = results[0].hotspot_score;
        assert!((actual - expected).abs() < 0.001,
            "Expected hotspot_score {expected:.3}, got {actual:.3}");
    }

    #[test]
    fn test_output_count_matches_input() {
        let files: Vec<String> = (0..5).map(|i| format!("file{i}.rs")).collect();
        let results = score_hotspots(
            &files,
            &zero_churn(&files), &zero_bugs(&files), &zero_reverts(&files),
            &zero_bursts(&files), &[], &zero_silo(&files), &zero_quality(&files),
            &DiffStatsMap::new(), &Weights::default(),
        );
        assert_eq!(results.len(), files.len(), "Output should have one entry per input file");
    }

    #[test]
    fn test_max_signals_produces_critical_tier() {
        let files = vec!["a.rs".to_string()];
        let mut churn   = zero_churn(&files);
        let mut bugs    = zero_bugs(&files);
        let mut reverts = zero_reverts(&files);
        let mut bursts  = zero_bursts(&files);
        let mut quality = zero_quality(&files);
        let mut silo    = zero_silo(&files);
        churn.get_mut("a.rs").unwrap().weighted_score = 100.0;
        bugs.get_mut("a.rs").unwrap().bug_score = 100.0;
        reverts.get_mut("a.rs").unwrap().revert_score = 100.0;
        bursts.get_mut("a.rs").unwrap().burst_score = 100.0;
        quality.get_mut("a.rs").unwrap().commit_quality_score = 100.0;
        silo.get_mut("a.rs").unwrap().top_author_percent = 100.0;
        let results = score_hotspots(
            &files, &churn, &bugs, &reverts, &bursts, &[], &silo, &quality,
            &DiffStatsMap::new(), &Weights::default(),
        );
        assert!(results[0].hotspot_score > 75.0, "All signals at max should produce CRITICAL score");
        assert_eq!(results[0].tier, Tier::Critical);
    }
}

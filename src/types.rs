use serde::Serialize;
use std::collections::HashMap;

// ─── Core Git Data ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Commit {
    #[allow(dead_code)]
    pub hash: String,
    pub author: String,
    pub timestamp: i64,
    pub subject: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
}

pub type DiffStatsMap = HashMap<String, DiffStats>;

// ─── Analyzer Outputs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ChurnData {
    pub commit_count: usize,
    pub weighted_score: f64,
    pub raw_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BugData {
    pub bug_commits: usize,
    pub bug_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevertData {
    pub revert_count: usize,
    pub revert_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BurstData {
    pub burst_incidents: usize,
    pub burst_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SiloData {
    pub top_author: String,
    pub top_author_percent: f64,
    pub author_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitQualityData {
    pub wip_commits: usize,
    pub large_commit_count: usize,
    pub commit_quality_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CouplingEntry {
    pub file_a: String,
    pub file_b: String,
    pub co_changes: usize,
    pub strength: f64,
}

// ─── Security ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SecurityRisk {
    pub file: String,
    pub risk_type: String,
    pub commit_count: usize,
    pub first_seen: String,
    pub last_seen: String,
}

// ─── Scoring ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Tier {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tier::Critical => write!(f, "CRITICAL"),
            Tier::High     => write!(f, "HIGH"),
            Tier::Medium   => write!(f, "MEDIUM"),
            Tier::Low      => write!(f, "LOW"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HotspotDetails {
    pub commit_count: usize,
    pub bug_commits: usize,
    pub revert_count: usize,
    pub burst_incidents: usize,
    pub wip_commits: usize,
    pub large_commit_count: usize,
    pub top_author: String,
    pub top_author_percent: f64,
    pub author_count: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct HotspotResult {
    pub file: String,
    pub hotspot_score: f64,
    pub churn_score: f64,
    pub bug_fix_score: f64,
    pub revert_score: f64,
    pub burst_score: f64,
    pub coupling_score: f64,
    pub silo_score: f64,
    pub commit_quality_score: f64,
    pub tier: Tier,
    pub details: HotspotDetails,
}

#[derive(Debug, Clone)]
pub struct Weights {
    pub churn: f64,
    pub bugs: f64,
    pub reverts: f64,
    pub bursts: f64,
    pub coupling: f64,
    pub silo: f64,
    pub commit_quality: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            churn:          0.27,
            bugs:           0.27,
            reverts:        0.14,
            bursts:         0.09,
            coupling:       0.09,
            silo:           0.05,
            commit_quality: 0.09,
        }
    }
}

// ─── Report ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ReportMeta {
    pub since: String,
    pub commit_count: usize,
    pub file_count: usize,
    pub analyzed_at: String,
    pub repo_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub meta: ReportMeta,
    pub results: Vec<HotspotResult>,
    pub couplings: Vec<CouplingEntry>,
    pub security_risks: Vec<SecurityRisk>,
}

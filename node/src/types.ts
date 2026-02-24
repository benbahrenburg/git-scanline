// ─── Core Git Data ────────────────────────────────────────────────────────────

export interface Commit {
  hash: string;
  author: string;
  timestamp: number;
  subject: string;
  files: string[];
}

export type DiffStatsMap = Map<string, { additions: number; deletions: number }>;

// ─── Analyzer Outputs (per-file Maps) ─────────────────────────────────────────

export interface ChurnData {
  commitCount: number;
  weightedScore: number;
  rawScore: number;
}

export interface BugData {
  bugCommits: number;
  bugScore: number;
}

export interface RevertData {
  revertCount: number;
  revertScore: number;
}

export interface BurstData {
  burstIncidents: number;
  burstScore: number;
}

export interface SiloData {
  topAuthor: string;
  topAuthorPercent: number;
  authorCount: number;
}

export interface CommitQualityData {
  wipCommits: number;
  largeCommitCount: number;
  commitQualityScore: number;
}

export interface CouplingEntry {
  fileA: string;
  fileB: string;
  coChanges: number;
  strength: number;
}

// ─── Security ─────────────────────────────────────────────────────────────────

export interface SecurityRisk {
  file: string;
  riskType: 'env-file' | 'key-or-cert' | 'credential-file';
  commitCount: number;
  firstSeen: string;
  lastSeen: string;
}

// ─── Scoring ──────────────────────────────────────────────────────────────────

export type Tier = 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW';

export interface HotspotDetails {
  commitCount: number;
  bugCommits: number;
  revertCount: number;
  burstIncidents: number;
  wipCommits: number;
  largeCommitCount: number;
  topAuthor: string;
  topAuthorPercent: number;
  authorCount: number;
  additions: number;
  deletions: number;
}

export interface HotspotResult {
  file: string;
  hotspotScore: number;
  churnScore: number;
  bugFixScore: number;
  revertScore: number;
  burstScore: number;
  couplingScore: number;
  siloScore: number;
  commitQualityScore: number;
  tier: Tier;
  details: HotspotDetails;
}

export interface Weights {
  churn: number;
  bugs: number;
  reverts: number;
  bursts: number;
  coupling: number;
  silo: number;
  commitQuality: number;
}

export interface ScoringInput {
  churnData: Map<string, ChurnData>;
  bugData: Map<string, BugData>;
  revertData: Map<string, RevertData>;
  burstData: Map<string, BurstData>;
  couplingData: CouplingEntry[];
  siloData: Map<string, SiloData>;
  commitQualityData: Map<string, CommitQualityData>;
  diffStats: DiffStatsMap;
  weights: Partial<Weights>;
}

// ─── Report ───────────────────────────────────────────────────────────────────

export interface ReportMeta {
  since: string;
  commitCount: number;
  fileCount: number;
  analyzedAt: string;
}

export interface Report {
  meta: ReportMeta;
  results: HotspotResult[];
  couplings: CouplingEntry[];
  securityRisks: SecurityRisk[];
}

import { getCouplingScores } from '../analyzers/coupling.js';
import type {
  ScoringInput,
  HotspotResult,
  HotspotDetails,
  Tier,
  Weights,
  ChurnData,
  BugData,
  RevertData,
  BurstData,
  SiloData,
  CommitQualityData,
} from '../types.js';

const DEFAULT_WEIGHTS: Weights = {
  churn:         0.27,
  bugs:          0.27,
  reverts:       0.14,
  bursts:        0.09,
  coupling:      0.09,
  silo:          0.05,
  commitQuality: 0.09,
};

const TIER_THRESHOLDS = {
  CRITICAL: 75,
  HIGH:     50,
  MEDIUM:   25,
} as const;

const CHURN_FALLBACK:  ChurnData         = { weightedScore: 0, commitCount: 0, rawScore: 0 };
const BUG_FALLBACK:    BugData           = { bugScore: 0, bugCommits: 0 };
const REVERT_FALLBACK: RevertData        = { revertScore: 0, revertCount: 0 };
const BURST_FALLBACK:  BurstData         = { burstScore: 0, burstIncidents: 0 };
const SILO_FALLBACK:   SiloData          = { topAuthor: 'unknown', topAuthorPercent: 0, authorCount: 1 };
const CQ_FALLBACK:     CommitQualityData = { wipCommits: 0, largeCommitCount: 0, commitQualityScore: 0 };
const DIFF_FALLBACK                      = { additions: 0, deletions: 0 };

/**
 * Aggregates all analyzer outputs into a final hotspot score (0–100) per file.
 * Weights are automatically normalized so they always sum to 1.
 */
export function scoreHotspots(
  files: string[],
  input: ScoringInput
): HotspotResult[] {
  const { churnData, bugData, revertData, burstData, couplingData,
          siloData, commitQualityData, diffStats, weights } = input;

  const w: Weights = { ...DEFAULT_WEIGHTS, ...weights };
  const weightSum = (Object.values(w) as number[]).reduce((a, b) => a + b, 0);
  (Object.keys(w) as (keyof Weights)[]).forEach(key => { w[key] = w[key] / weightSum; });

  const couplingScores = getCouplingScores(files, couplingData);

  return files.map((file): HotspotResult => {
    const churn   = churnData.get(file)         ?? CHURN_FALLBACK;
    const bugs    = bugData.get(file)           ?? BUG_FALLBACK;
    const reverts = revertData.get(file)        ?? REVERT_FALLBACK;
    const bursts  = burstData.get(file)         ?? BURST_FALLBACK;
    const silo    = siloData.get(file)          ?? SILO_FALLBACK;
    const cq      = commitQualityData.get(file) ?? CQ_FALLBACK;
    const diff    = diffStats.get(file)         ?? DIFF_FALLBACK;

    const churnScore         = churn.weightedScore;
    const bugFixScore        = bugs.bugScore;
    const revertScore        = reverts.revertScore;
    const burstScore         = bursts.burstScore;
    const couplingScore      = couplingScores.get(file) ?? 0;
    const siloScore          = silo.topAuthorPercent;
    const commitQualityScore = cq.commitQualityScore;

    const hotspotScore = Math.round(
      churnScore         * w.churn         +
      bugFixScore        * w.bugs          +
      revertScore        * w.reverts       +
      burstScore         * w.bursts        +
      couplingScore      * w.coupling      +
      siloScore          * w.silo          +
      commitQualityScore * w.commitQuality
    );

    const details: HotspotDetails = {
      commitCount:      churn.commitCount,
      bugCommits:       bugs.bugCommits,
      revertCount:      reverts.revertCount,
      burstIncidents:   bursts.burstIncidents,
      wipCommits:       cq.wipCommits,
      largeCommitCount: cq.largeCommitCount,
      topAuthor:        silo.topAuthor,
      topAuthorPercent: silo.topAuthorPercent,
      authorCount:      silo.authorCount,
      additions:        diff.additions,
      deletions:        diff.deletions,
    };

    return {
      file,
      hotspotScore,
      churnScore,
      bugFixScore,
      revertScore,
      burstScore,
      couplingScore,
      siloScore,
      commitQualityScore,
      tier: getTier(hotspotScore),
      details,
    };
  });
}

function getTier(score: number): Tier {
  if (score >= TIER_THRESHOLDS.CRITICAL) return 'CRITICAL';
  if (score >= TIER_THRESHOLDS.HIGH)     return 'HIGH';
  if (score >= TIER_THRESHOLDS.MEDIUM)   return 'MEDIUM';
  return 'LOW';
}

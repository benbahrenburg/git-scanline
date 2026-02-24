import dayjs from 'dayjs';
import type { Commit, ChurnData } from '../types.js';

// Exponential decay constant. λ = 0.005 → half-life ≈ 139 days.
// A commit from yesterday weighs ~1.0; one from 9 months ago weighs ~0.25.
const DECAY_LAMBDA = 0.005;

/**
 * Calculates churn rate with recency decay per file.
 *
 * Returns: Map<filename, ChurnData>
 *   - commitCount:   raw number of commits touching this file
 *   - weightedScore: 0–100, recent commits weighted higher (exponential decay)
 *   - rawScore:      0–100, simple commit-frequency score
 */
export function analyzeChurn(
  commits: Commit[],
  files: string[]
): Map<string, ChurnData> {
  const fileSet = new Set(files);
  const now = dayjs();

  // filename → { commitCount, weightedChurn }
  const fileChurn = new Map<string, { commitCount: number; weightedChurn: number }>();

  for (const commit of commits) {
    const daysAgo = now.diff(dayjs.unix(commit.timestamp), 'day');
    const decayWeight = Math.exp(-DECAY_LAMBDA * daysAgo);

    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;

      const existing = fileChurn.get(file) ?? { commitCount: 0, weightedChurn: 0 };
      fileChurn.set(file, {
        commitCount:   existing.commitCount + 1,
        weightedChurn: existing.weightedChurn + decayWeight,
      });
    }
  }

  const totalCommits = Math.max(commits.length, 1);
  const maxWeighted = [...fileChurn.values()].reduce(
    (max, v) => Math.max(max, v.weightedChurn), 0.0001
  );

  const result = new Map<string, ChurnData>();

  for (const file of files) {
    const data = fileChurn.get(file) ?? { commitCount: 0, weightedChurn: 0 };
    result.set(file, {
      commitCount:   data.commitCount,
      rawScore:      Math.min(100, Math.round((data.commitCount / totalCommits) * 500)),
      weightedScore: Math.round((data.weightedChurn / maxWeighted) * 100),
    });
  }

  return result;
}

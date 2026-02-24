import type { Commit, CommitQualityData } from '../types.js';

/**
 * Matches commit messages that suggest low-quality or rushed work:
 * - WIP / temp / draft indicators
 * - Very generic single-word subjects
 * - Fixup / squash (rebasing signals)
 */
const WIP_PATTERN = /\b(wip|temp|tmp|fixup|squash|hack|dirty|oops|typo|debug|draft)\b|^(fix|update|changes|stuff|misc|test|cleanup|commit|save|ok|done)\s*[.!]?\s*$/i;

/** Commits touching more than this many files are "large" (e.g. mass reformats, merges). */
const LARGE_COMMIT_THRESHOLD = 30;

/** Subjects shorter than this (after trimming) are flagged as low-quality. */
const SHORT_MSG_MIN_LENGTH = 10;

export function analyzeCommitQuality(
  commits: Commit[],
  files: string[]
): Map<string, CommitQualityData> {
  const fileSet = new Set(files);
  const wipCounts   = new Map<string, number>();
  const largeCounts = new Map<string, number>();

  for (const commit of commits) {
    const isWip   = WIP_PATTERN.test(commit.subject.trim()) ||
                    commit.subject.trim().length < SHORT_MSG_MIN_LENGTH;
    const isLarge = commit.files.length > LARGE_COMMIT_THRESHOLD;

    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;
      if (isWip)   wipCounts.set(file,   (wipCounts.get(file)   ?? 0) + 1);
      if (isLarge) largeCounts.set(file, (largeCounts.get(file) ?? 0) + 1);
    }
  }

  // Normalize scores: most-affected file = 100
  const maxWip   = Math.max(...wipCounts.values(),   1);
  const maxLarge = Math.max(...largeCounts.values(),  1);

  return new Map(files.map(file => {
    const wip   = wipCounts.get(file)   ?? 0;
    const large = largeCounts.get(file) ?? 0;
    // WIP commits weighted more heavily (60/40) — they signal rushed work
    const score = (wip / maxWip) * 60 + (large / maxLarge) * 40;
    return [file, { wipCommits: wip, largeCommitCount: large, commitQualityScore: score }];
  }));
}

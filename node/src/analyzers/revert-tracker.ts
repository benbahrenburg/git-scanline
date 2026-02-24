import type { Commit, RevertData } from '../types.js';

const REVERT_PATTERN = /^revert\b/i;

/**
 * Detects files that appear in revert commits.
 * Reverts are a strong signal: they mean a change introduced a bug serious
 * enough to be rolled back.
 *
 * Returns: Map<filename, RevertData>
 */
export function analyzeReverts(
  commits: Commit[],
  files: string[]
): Map<string, RevertData> {
  const fileSet = new Set(files);
  const revertCommits = commits.filter(c => REVERT_PATTERN.test(c.subject.trim()));

  // filename → revert commit count
  const fileReverts = new Map<string, number>();

  for (const commit of revertCommits) {
    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;
      fileReverts.set(file, (fileReverts.get(file) ?? 0) + 1);
    }
  }

  const maxReverts = [...fileReverts.values()].reduce(
    (max, v) => Math.max(max, v), 0.0001
  );

  const result = new Map<string, RevertData>();
  for (const file of files) {
    const count = fileReverts.get(file) ?? 0;
    result.set(file, {
      revertCount: count,
      revertScore: Math.round((count / maxReverts) * 100),
    });
  }

  return result;
}

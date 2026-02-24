import type { Commit, BugData } from '../types.js';

// Matches common bug-fix vocabulary in commit subjects
const BUG_PATTERN = /\b(fix|bug|patch|hotfix|regression|broken|crash|defect|issue|error)\b/i;

/**
 * Identifies files that frequently appear in bug-fix commits.
 *
 * Returns: Map<filename, BugData>
 *   - bugCommits: number of bug-tagged commits that touched this file
 *   - bugScore:   0–100, normalized relative to the most-bug-correlated file
 */
export function analyzeBugCorrelation(
  commits: Commit[],
  files: string[]
): Map<string, BugData> {
  const fileSet = new Set(files);
  const bugCommits = commits.filter(c => BUG_PATTERN.test(c.subject));

  // filename → count of bug commits touching it
  const fileBugCounts = new Map<string, number>();

  for (const commit of bugCommits) {
    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;
      fileBugCounts.set(file, (fileBugCounts.get(file) ?? 0) + 1);
    }
  }

  const maxCount = [...fileBugCounts.values()].reduce(
    (max, v) => Math.max(max, v), 0.0001
  );

  const result = new Map<string, BugData>();
  for (const file of files) {
    const count = fileBugCounts.get(file) ?? 0;
    result.set(file, {
      bugCommits: count,
      bugScore:   Math.round((count / maxCount) * 100),
    });
  }

  return result;
}

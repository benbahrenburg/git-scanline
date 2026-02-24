import type { Commit, CouplingEntry } from '../types.js';

// Skip commits touching more than this many files — they're almost always
// large merges or reformats and would produce O(k²) pairs that skew results.
const MAX_FILES_PER_COMMIT = 20;

/**
 * Builds a co-change coupling matrix.
 *
 * Files that always change together but aren't structurally related (e.g.,
 * `auth.js` and `payments.js`) suggest hidden dependencies — a major source
 * of unexpected bugs.
 *
 * Coupling strength is Jaccard similarity:
 *   strength = coChanges / (commitsA + commitsB - coChanges)
 *
 * Returns: CouplingEntry[] sorted by coChanges desc
 */
export function analyzeCoupling(
  commits: Commit[],
  files: string[]
): CouplingEntry[] {
  const fileSet = new Set(files);

  // "fileA||fileB" (sorted) → co-change count
  const pairCounts = new Map<string, number>();

  // filename → total commits touching it
  const fileCounts = new Map<string, number>();

  for (const commit of commits) {
    const touched = commit.files.filter(f => fileSet.has(f));

    for (const file of touched) {
      fileCounts.set(file, (fileCounts.get(file) ?? 0) + 1);
    }

    // Skip large-fanout commits (merges, reformats) — they'd produce O(k²)
    // pairs that OOM on large repos and aren't meaningful coupling signals.
    if (touched.length > MAX_FILES_PER_COMMIT) continue;

    for (let i = 0; i < touched.length; i++) {
      for (let j = i + 1; j < touched.length; j++) {
        const key = [touched[i], touched[j]].sort().join('||');
        pairCounts.set(key, (pairCounts.get(key) ?? 0) + 1);
      }
    }
  }

  const couplings: CouplingEntry[] = [];

  for (const [key, coChanges] of pairCounts) {
    if (coChanges < 3) continue; // Ignore very rare co-changes

    const [fileA, fileB] = key.split('||') as [string, string];
    const totalA = fileCounts.get(fileA) ?? 1;
    const totalB = fileCounts.get(fileB) ?? 1;

    // Jaccard: how often they change together vs. independently
    const union = totalA + totalB - coChanges;
    const strength = union > 0 ? Math.round((coChanges / union) * 100) : 0;

    couplings.push({ fileA, fileB, coChanges, strength });
  }

  return couplings.sort((a, b) => b.coChanges - a.coChanges);
}

/**
 * Converts the coupling array into a per-file score map.
 * Each file gets the maximum coupling strength it has with any other file.
 *
 * Returns: Map<filename, couplingScore 0–100>
 */
export function getCouplingScores(
  files: string[],
  couplings: CouplingEntry[]
): Map<string, number> {
  const scores = new Map<string, number>(files.map(f => [f, 0]));

  for (const { fileA, fileB, strength } of couplings) {
    const prevA = scores.get(fileA) ?? 0;
    const prevB = scores.get(fileB) ?? 0;
    if (scores.has(fileA)) scores.set(fileA, Math.max(prevA, strength));
    if (scores.has(fileB)) scores.set(fileB, Math.max(prevB, strength));
  }

  return scores;
}

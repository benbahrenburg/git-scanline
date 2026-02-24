import type { Commit, BurstData } from '../types.js';

// A "burst" = BURST_MIN_COMMITS or more commits to the same file within BURST_WINDOW_HOURS
const BURST_WINDOW_HOURS = 24;
const BURST_MIN_COMMITS  = 3;

/**
 * Detects rapid successive commit bursts per file.
 *
 * Burst patterns (patch-on-patch) indicate a file where bugs are being fixed
 * messily — someone pushed a fix, it was wrong, they pushed another, and so on.
 *
 * Returns: Map<filename, BurstData>
 */
export function analyzeBursts(
  commits: Commit[],
  files: string[]
): Map<string, BurstData> {
  const fileSet = new Set(files);
  const burstWindowSeconds = BURST_WINDOW_HOURS * 3600;

  // filename → sorted array of commit timestamps
  const fileTimestamps = new Map<string, number[]>();

  for (const commit of commits) {
    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;
      const arr = fileTimestamps.get(file) ?? [];
      arr.push(commit.timestamp);
      fileTimestamps.set(file, arr);
    }
  }

  // filename → raw burst incident count (before normalization)
  const rawResult = new Map<string, number>();

  for (const file of files) {
    const timestamps = (fileTimestamps.get(file) ?? []).sort((a, b) => a - b);
    let burstIncidents = 0;
    let i = 0;

    while (i < timestamps.length) {
      let count = 1;
      let j = i + 1;
      while (
        j < timestamps.length &&
        (timestamps[j] ?? 0) - (timestamps[i] ?? 0) <= burstWindowSeconds
      ) {
        count++;
        j++;
      }

      if (count >= BURST_MIN_COMMITS) {
        burstIncidents++;
        i = j; // Skip past this burst window
      } else {
        i++;
      }
    }

    rawResult.set(file, burstIncidents);
  }

  const maxBursts = [...rawResult.values()].reduce(
    (max, v) => Math.max(max, v), 0.0001
  );

  const result = new Map<string, BurstData>();
  for (const [file, burstIncidents] of rawResult) {
    result.set(file, {
      burstIncidents,
      burstScore: Math.round((burstIncidents / maxBursts) * 100),
    });
  }

  return result;
}

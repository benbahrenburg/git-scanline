import { execSync } from 'child_process';
import type { DiffStatsMap } from '../types.js';

/**
 * Runs `git log --numstat` and accumulates line-level churn per file.
 *
 * Returns: Map<filename, { additions: number, deletions: number }>
 */
export function parseDiff(
  cwd: string,
  since: string = '12 months ago',
  pathFilter: string | null = null
): DiffStatsMap {
  const sinceArg = since ? `--since="${since}"` : '';
  const pathArg = pathFilter ? `-- "${pathFilter}"` : '';

  let output: string;
  try {
    output = execSync(
      `git log --format="COMMIT|%H" --numstat ${sinceArg} ${pathArg}`,
      { cwd, encoding: 'utf8', maxBuffer: 200 * 1024 * 1024 }
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`git numstat failed: ${message}`);
  }

  return parseNumstatOutput(output);
}

function parseNumstatOutput(output: string): DiffStatsMap {
  const stats: DiffStatsMap = new Map();

  for (const line of output.split('\n')) {
    const trimmed = line.trim();

    // Skip blank lines and commit header lines
    if (!trimmed || trimmed.startsWith('COMMIT|')) continue;

    // numstat format: <additions>\t<deletions>\t<filename>
    const parts = trimmed.split('\t');
    if (parts.length < 3) continue;

    // Binary files are shown as '-'
    if (parts[0] === '-' || parts[1] === '-') continue;

    const additions = parseInt(parts[0] ?? '0', 10);
    const deletions = parseInt(parts[1] ?? '0', 10);
    if (isNaN(additions) || isNaN(deletions)) continue;

    const rawFilename = parts[2];
    if (!rawFilename) continue;

    const filename = normalizeNumstatFilename(rawFilename);
    if (!filename) continue;

    const existing = stats.get(filename) ?? { additions: 0, deletions: 0 };
    stats.set(filename, {
      additions: existing.additions + additions,
      deletions: existing.deletions + deletions,
    });
  }

  return stats;
}

function normalizeNumstatFilename(raw: string): string | null {
  // Handle "src/{old => new}/file.js"
  if (raw.includes('{') && raw.includes('=>')) {
    const normalized = raw
      .replace(/\{[^}]+ => ([^}]+)\}/, '$1')
      .replace(/\/\//g, '/');
    return normalized.includes('{') ? null : normalized.trim();
  }

  // Handle "old-name => new-name"
  if (raw.includes(' => ')) {
    return raw.split(' => ').pop()?.trim() ?? null;
  }

  return raw.trim() || null;
}

import { execSync } from 'child_process';
import type { Commit } from '../types.js';

/**
 * Runs `git log` and parses it into structured commit objects.
 *
 * Each commit: { hash, author, timestamp, subject, files[] }
 *
 * Uses a "COMMIT|" prefix on the format line so we can reliably distinguish
 * commit headers from file-name lines in the mixed --name-only output.
 */
export function parseLog(
  cwd: string,
  since: string = '',
  pathFilter: string | null = null
): Commit[] {
  const sinceArg = since ? `--since="${since}"` : '';
  const pathArg = pathFilter ? `-- "${pathFilter}"` : '';

  let output: string;
  try {
    output = execSync(
      `git log --format="COMMIT|%H|%ae|%ad|%s" --date=unix --name-only --diff-filter=ACDMRT ${sinceArg} ${pathArg}`,
      { cwd, encoding: 'utf8', maxBuffer: 200 * 1024 * 1024 }
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`git log failed: ${message}`);
  }

  return parseCommitOutput(output);
}

function parseCommitOutput(output: string): Commit[] {
  const commits: Commit[] = [];
  let current: Commit | null = null;

  for (const line of output.split('\n')) {
    const trimmed = line.trim();

    if (trimmed.startsWith('COMMIT|')) {
      if (current) commits.push(current);
      const parts = trimmed.split('|');
      current = {
        hash:      parts[1] ?? '',
        author:    parts[2] ?? '',
        timestamp: parseInt(parts[3] ?? '0', 10),
        subject:   parts.slice(4).join('|'), // re-join in case subject contains '|'
        files:     [],
      };
    } else if (trimmed && current) {
      const file = normalizeFilename(trimmed);
      if (file) current.files.push(file);
    }
  }

  if (current) commits.push(current);
  return commits;
}

function normalizeFilename(raw: string): string | null {
  // Handle "src/{old-dir => new-dir}/file.js" → "src/new-dir/file.js"
  if (raw.includes('{') && raw.includes('=>')) {
    const normalized = raw
      .replace(/\{[^}]+ => ([^}]+)\}/, '$1')
      .replace(/\/\//g, '/');
    if (!normalized.includes('{')) return normalized.trim();
    return null;
  }
  // Handle plain "old-name => new-name"
  if (raw.includes(' => ')) {
    return raw.split(' => ').pop()?.trim() ?? null;
  }
  return raw;
}

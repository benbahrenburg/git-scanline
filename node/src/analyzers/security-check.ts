import type { Commit, SecurityRisk } from '../types.js';

// .env, .env.production, .env.local, etc.
const ENV_PATTERN = /(?:^|\/)\.(env)(\.|$)/i;

// Private keys, certificates, keystores
const KEY_PATTERN = /\.(pem|key|p12|pfx|cer|crt|jks|ppk|keystore)$/i;

// Files explicitly named after credentials
const CRED_PATTERN = /(?:^|\/)(?:credential|secret|password|passwd|private[_-]?key|api[_-]?key|auth[_-]?token)[^/]*$/i;

function getRiskType(file: string): SecurityRisk['riskType'] | null {
  if (ENV_PATTERN.test(file))  return 'env-file';
  if (KEY_PATTERN.test(file))  return 'key-or-cert';
  if (CRED_PATTERN.test(file)) return 'credential-file';
  return null;
}

/**
 * Scans raw (unfiltered) commits for security-sensitive files that were
 * ever committed to git history. Even deleted files are flagged because
 * they remain accessible via git history.
 */
export function analyzeSecurityRisks(commits: Commit[]): SecurityRisk[] {
  const risks = new Map<string, {
    riskType: SecurityRisk['riskType'];
    count: number;
    first: number;
    last: number;
  }>();

  for (const commit of commits) {
    for (const file of commit.files) {
      const riskType = getRiskType(file);
      if (!riskType) continue;

      const existing = risks.get(file);
      if (!existing) {
        risks.set(file, { riskType, count: 1, first: commit.timestamp, last: commit.timestamp });
      } else {
        existing.count++;
        if (commit.timestamp < existing.first) existing.first = commit.timestamp;
        if (commit.timestamp > existing.last)  existing.last  = commit.timestamp;
      }
    }
  }

  return [...risks.entries()]
    .map(([file, d]) => ({
      file,
      riskType:    d.riskType,
      commitCount: d.count,
      firstSeen:   new Date(d.first * 1000).toISOString().slice(0, 10),
      lastSeen:    new Date(d.last  * 1000).toISOString().slice(0, 10),
    }))
    .sort((a, b) => b.commitCount - a.commitCount);
}

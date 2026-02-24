/**
 * git-scanline — Integration Tests
 *
 * Reads TEST_REPO_PATH from the workspace-root .env file (or the environment).
 * Tests that require a real git repository are automatically skipped when
 * TEST_REPO_PATH is not set or the path does not exist.
 *
 * Setup:
 *   1. Copy /path/to/workspace/.env.example → .env  (or edit the existing .env)
 *   2. Set TEST_REPO_PATH to any local git repository
 *   3. Run:  npm run build && npm test
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// ── Load .env from workspace root ──────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url));
const WORKSPACE_ROOT = join(__dirname, '../../../');  // node/tests → node → workspace

function loadEnv(): Record<string, string> {
  const envPath = join(WORKSPACE_ROOT, '.env');
  if (!existsSync(envPath)) return {};
  const result: Record<string, string> = {};
  for (const line of readFileSync(envPath, 'utf8').split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    const val = trimmed.slice(eqIdx + 1).trim().replace(/^['"]|['"]$/g, '');
    result[key] = val;
  }
  return result;
}

const env = loadEnv();
const TEST_REPO = (env['TEST_REPO_PATH'] ?? process.env['TEST_REPO_PATH'] ?? '').trim();
const hasRepo = TEST_REPO.length > 0 && existsSync(TEST_REPO);

// ── Pure-function unit tests (no repo needed) ──────────────────────────────────

test('filterFiles removes package.json and lock files', async () => {
  const { filterFiles } = await import('../src/filters/file-filter.js');
  const files = [
    'src/app.ts',
    'package.json',
    'yarn.lock',
    'package-lock.json',
    'npm-shrinkwrap.json',
    'src/lib.ts',
    '.DS_Store',
    'node_modules/lodash/index.js',
  ];
  const filtered = filterFiles(files, null);
  assert.ok(!filtered.includes('package.json'), 'package.json must be filtered');
  assert.ok(!filtered.includes('yarn.lock'), 'yarn.lock must be filtered');
  assert.ok(!filtered.includes('package-lock.json'), 'package-lock.json must be filtered');
  assert.ok(!filtered.includes('npm-shrinkwrap.json'), 'npm-shrinkwrap.json must be filtered');
  assert.ok(!filtered.includes('.DS_Store'), '.DS_Store must be filtered');
  assert.ok(!filtered.some(f => f.startsWith('node_modules/')), 'node_modules must be filtered');
  assert.ok(filtered.includes('src/app.ts'), 'src/app.ts must be kept');
  assert.ok(filtered.includes('src/lib.ts'), 'src/lib.ts must be kept');
});

test('filterFiles respects path filter', async () => {
  const { filterFiles } = await import('../src/filters/file-filter.js');
  const files = ['src/app.ts', 'tests/foo.ts', 'lib/util.ts'];
  const filtered = filterFiles(files, 'src');
  assert.ok(filtered.includes('src/app.ts'), 'src/ file should be included');
  assert.ok(!filtered.includes('tests/foo.ts'), 'tests/ file should be excluded');
});

test('analyzeSecurityRisks detects env files', async () => {
  const { analyzeSecurityRisks } = await import('../src/analyzers/security-check.js');
  const fakeCommits = [
    { hash: 'abc', author: 'dev@example.com', timestamp: 1700000000, subject: 'add config', files: ['.env', '.env.production', 'src/app.ts'] },
    { hash: 'def', author: 'dev@example.com', timestamp: 1700001000, subject: 'add key', files: ['server.key', 'certs/cert.pem'] },
  ];
  const risks = analyzeSecurityRisks(fakeCommits as any);
  const files = risks.map(r => r.file);
  assert.ok(files.includes('.env'), '.env should be flagged');
  assert.ok(files.includes('.env.production'), '.env.production should be flagged');
  assert.ok(files.includes('server.key'), 'server.key should be flagged');
  assert.ok(files.includes('certs/cert.pem'), 'cert.pem should be flagged');
  assert.ok(!files.includes('src/app.ts'), 'src/app.ts must not be flagged');
});

test('hotspot score is within 0–100 range for synthetic data', async () => {
  const { scoreHotspots } = await import('../src/scoring/hotspot-scorer.js');
  const files = ['src/a.ts', 'src/b.ts', 'src/c.ts'];
  const empty = new Map();
  const results = scoreHotspots(files, {
    churnData: empty,
    bugData: empty,
    revertData: empty,
    burstData: empty,
    couplingData: [],
    siloData: empty,
    commitQualityData: empty,
    diffStats: empty,
    weights: { churn: 0.27, bugs: 0.27, reverts: 0.14, bursts: 0.09, coupling: 0.09, silo: 0.05, commitQuality: 0.09 },
  });
  assert.equal(results.length, files.length);
  for (const r of results) {
    assert.ok(r.hotspotScore >= 0 && r.hotspotScore <= 100,
      `Score out of range: ${r.hotspotScore} for ${r.file}`);
  }
});

// ── Integration tests (require TEST_REPO_PATH) ─────────────────────────────────

test('parseLog returns commits for real repo', { skip: !hasRepo }, async () => {
  const { parseLog } = await import('../src/git/log-parser.js');
  const commits = parseLog(TEST_REPO, '', null);
  assert.ok(commits.length > 0, 'Should find commits in test repo');
  assert.ok(commits[0]!.hash.length > 0, 'Commit should have a hash');
  assert.ok(commits[0]!.author.length > 0, 'Commit should have an author');
  assert.ok(commits[0]!.timestamp > 0, 'Commit should have a timestamp');
});

test('full pipeline produces valid hotspot scores for real repo', { skip: !hasRepo }, async () => {
  const { parseLog } = await import('../src/git/log-parser.js');
  const { parseDiff } = await import('../src/git/diff-parser.js');
  const { filterFiles } = await import('../src/filters/file-filter.js');
  const { analyzeChurn } = await import('../src/analyzers/churn.js');
  const { analyzeBugCorrelation } = await import('../src/analyzers/bug-correlation.js');
  const { analyzeReverts } = await import('../src/analyzers/revert-tracker.js');
  const { analyzeBursts } = await import('../src/analyzers/burst-detector.js');
  const { analyzeCoupling } = await import('../src/analyzers/coupling.js');
  const { analyzeAuthors } = await import('../src/git/blame-analyzer.js');
  const { analyzeCommitQuality } = await import('../src/analyzers/commit-quality.js');
  const { scoreHotspots } = await import('../src/scoring/hotspot-scorer.js');

  const commits = parseLog(TEST_REPO, '', null);
  assert.ok(commits.length > 0, 'Repo must have commits');

  const allFiles = [...new Set(commits.flatMap(c => c.files))];
  const files = filterFiles(allFiles, null).slice(0, 50); // limit for speed
  if (files.length === 0) return; // very unusual — skip quietly

  const diffStats = parseDiff(TEST_REPO, '', null);

  const results = scoreHotspots(files, {
    churnData: analyzeChurn(commits, files),
    bugData: analyzeBugCorrelation(commits, files),
    revertData: analyzeReverts(commits, files),
    burstData: analyzeBursts(commits, files),
    couplingData: analyzeCoupling(commits, files),
    siloData: analyzeAuthors(commits, files),
    commitQualityData: analyzeCommitQuality(commits, files),
    diffStats,
    weights: { churn: 0.27, bugs: 0.27, reverts: 0.14, bursts: 0.09, coupling: 0.09, silo: 0.05, commitQuality: 0.09 },
  });

  assert.ok(results.length > 0, 'Should produce hotspot results');
  for (const r of results) {
    assert.ok(r.hotspotScore >= 0 && r.hotspotScore <= 100,
      `Score ${r.hotspotScore} out of range for ${r.file}`);
  }
});

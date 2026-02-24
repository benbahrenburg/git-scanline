#!/usr/bin/env node

import { program } from 'commander';
import ora, { type Ora } from 'ora';
import readline from 'readline';
import { readFileSync, readdirSync, statSync, existsSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join, basename, extname } from 'path';

import { parseLog } from '../src/git/log-parser.js';
import { parseDiff } from '../src/git/diff-parser.js';
import { analyzeAuthors } from '../src/git/blame-analyzer.js';
import { analyzeChurn } from '../src/analyzers/churn.js';
import { analyzeBugCorrelation } from '../src/analyzers/bug-correlation.js';
import { analyzeReverts } from '../src/analyzers/revert-tracker.js';
import { analyzeCoupling } from '../src/analyzers/coupling.js';
import { analyzeBursts } from '../src/analyzers/burst-detector.js';
import { analyzeCommitQuality } from '../src/analyzers/commit-quality.js';
import { analyzeSecurityRisks } from '../src/analyzers/security-check.js';
import { scoreHotspots } from '../src/scoring/hotspot-scorer.js';
import { filterFiles } from '../src/filters/file-filter.js';
import { reportTerminal } from '../src/reporters/terminal.js';
import { reportJson } from '../src/reporters/json.js';
import { reportHtml } from '../src/reporters/html.js';
import { startZorpAnimation, printZorpFooter, type ZorpHandle } from '../src/reporters/zorp-animation.js';

interface CliOptions {
  since: string;
  path?: string;
  top: string;
  bugsOnly: boolean;
  format: string;
  output?: string;
  noInteractive: boolean;
  weightChurn: string;
  weightBugs: string;
  weightReverts: string;
  weightBursts: string;
  weightCoupling: string;
  weightSilo: string;
  weightCommitQuality: string;
}

interface RunConfig {
  inputPath: string;
  since: string;
  pathFilter: string | null;
  topN: number;
  format: string;
  outputBase: string | null;
  bugsOnly: boolean;
  weights: {
    churn: number; bugs: number; reverts: number;
    bursts: number; coupling: number; silo: number; commitQuality: number;
  };
}


const __dirname = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(
  readFileSync(join(__dirname, '../../package.json'), 'utf8')
) as { version: string };

program
  .name('git-scanline')
  .description('Scan git history to surface bug-prone code hotspots.\n' +
    'Pass a git repo OR a parent folder — all nested git repos are analyzed.')
  .version(pkg.version)
  .argument('[path]', 'Path to a git repo or parent folder (omit for interactive mode)')
  .option('--since <date>',             'Analyze commits since this date (leave empty for all history)', '')
  .option('--path <path>',              'Focus on a specific subdirectory')
  .option('--top <n>',                  'Show top N files', '20')
  .option('--bugs-only',                'Only show files with bug-fix correlation', false)
  .option('--format <format>',          'Output format: terminal, json, html', 'terminal')
  .option('--output <file>',            'Output base path (repo name appended for multi-repo)')
  .option('--no-interactive',           'Skip interactive setup when no path is given')
  .option('--weight-churn <w>',         'Weight for churn score (0–1)', '0.27')
  .option('--weight-bugs <w>',          'Weight for bug fix score (0–1)', '0.27')
  .option('--weight-reverts <w>',       'Weight for revert score (0–1)', '0.14')
  .option('--weight-bursts <w>',        'Weight for burst score (0–1)', '0.09')
  .option('--weight-coupling <w>',      'Weight for coupling score (0–1)', '0.09')
  .option('--weight-silo <w>',          'Weight for silo score (0–1)', '0.05')
  .option('--weight-commit-quality <w>','Weight for commit quality score (0–1)', '0.09')
  .parse(process.argv);

const opts = program.opts<CliOptions>();
const positionalPath = program.args[0];

// ── Git repo discovery ─────────────────────────────────────────────────────────

const SKIP_SCAN_DIRS = new Set([
  'node_modules', 'vendor', 'target', 'dist', 'build',
  '.cache', '.git', '__pycache__', '.npm', '.yarn',
]);

function findGitRepos(root: string, depth = 0, maxDepth = 6): string[] {
  if (depth > maxDepth) return [];
  try {
    if (existsSync(join(root, '.git'))) return [root];
    const repos: string[] = [];
    for (const entry of readdirSync(root, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (entry.name.startsWith('.') || SKIP_SCAN_DIRS.has(entry.name)) continue;
      repos.push(...findGitRepos(join(root, entry.name), depth + 1, maxDepth));
    }
    return repos.sort();
  } catch { return []; }
}

// ── Output path helpers ────────────────────────────────────────────────────────

/** Insert repo name before extension: `report.html` + `my-app` → `report-my-app.html` */
function makeOutputPath(base: string, repoName: string): string {
  const ext  = extname(base);
  const stem = basename(base, ext);
  const dir  = dirname(base);
  const safe = repoName.replace(/[^a-zA-Z0-9_-]/g, '-');
  return join(dir, `${stem}-${safe}${ext}`);
}

// ── Analysis pipeline ──────────────────────────────────────────────────────────

async function runAnalysis(
  repoPath: string,
  config: RunConfig,
  spinner: Ora,
  isMulti: boolean
): Promise<void> {
  const repoName = basename(repoPath);
  const pfx = isMulti ? `[${repoName}] ` : '';
  const totalStart = Date.now();
  let stepStart = Date.now();
  let lastN = 0;
  let lastMsg = '';

  const fmtMs = (ms: number) => ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;

  const completeStep = () => {
    if (lastN > 0) {
      const t = fmtMs(Date.now() - stepStart);
      spinner.succeed(`${pfx}[${lastN}/9] ${lastMsg.padEnd(46)} ${t}`);
      spinner.start();
    }
  };

  const step = (n: number, msg: string) => {
    completeStep();
    lastN = n;
    lastMsg = msg;
    stepStart = Date.now();
    spinner.text = `${pfx}[${n}/9] ${msg}`;
  };

  step(1, 'Parsing commit log...');
  const commits = parseLog(repoPath, config.since, config.pathFilter);
  if (commits.length === 0) {
    spinner.warn(`${repoName}: No commits found — skipping (try --since="4 years ago")`);
    return;
  }

  step(2, 'Scanning for security risks...');
  const securityRisks = analyzeSecurityRisks(commits);

  step(3, 'Analyzing line-level churn...');
  const diffStats = parseDiff(repoPath, config.since, config.pathFilter);

  const allFiles = new Set<string>();
  for (const commit of commits) for (const file of commit.files) allFiles.add(file);

  step(4, 'Filtering files...');
  const filteredFiles = filterFiles([...allFiles], config.pathFilter);
  if (filteredFiles.length === 0) {
    spinner.warn(`${repoName}: No files after filtering — skipping`);
    return;
  }

  step(5, 'Analyzing churn patterns...');
  const churnData = analyzeChurn(commits, filteredFiles);

  step(6, 'Analyzing bug-fix correlations & reverts...');
  const bugData    = analyzeBugCorrelation(commits, filteredFiles);
  const revertData = analyzeReverts(commits, filteredFiles);

  step(7, 'Analyzing bursts & co-change coupling...');
  const burstData    = analyzeBursts(commits, filteredFiles);
  const couplingData = analyzeCoupling(commits, filteredFiles);

  step(8, 'Analyzing author concentration & commit quality...');
  const siloData          = analyzeAuthors(commits, filteredFiles);
  const commitQualityData = analyzeCommitQuality(commits, filteredFiles);

  step(9, 'Scoring hotspots...');
  let results = scoreHotspots(filteredFiles, {
    churnData, bugData, revertData, burstData, couplingData,
    siloData, commitQualityData, diffStats, weights: config.weights,
  })
    .sort((a, b) => b.hotspotScore - a.hotspotScore)
    .slice(0, config.topN);

  if (config.bugsOnly) results = results.filter(r => r.details.bugCommits > 0);

  const topCouplings = couplingData
    .filter(c => filteredFiles.includes(c.fileA) && filteredFiles.includes(c.fileB))
    .sort((a, b) => b.coChanges - a.coChanges)
    .slice(0, 10);

  completeStep(); // complete step 9
  const totalTime = fmtMs(Date.now() - totalStart);
  spinner.succeed(
    `${repoName}: ${commits.length} commits, ${filteredFiles.length} files — ⏱ ${totalTime}` +
    (securityRisks.length > 0 ? ` — ⚠ ${securityRisks.length} security risk(s)` : '')
  );

  const report = {
    meta: {
      since: config.since || 'all history',
      commitCount: commits.length,
      fileCount: filteredFiles.length,
      analyzedAt: new Date().toISOString(),
    },
    results,
    couplings: topCouplings,
    securityRisks,
  };

  // Resolve output path for this repo
  const outputPath = config.outputBase
    ? (isMulti ? makeOutputPath(config.outputBase, repoName) : config.outputBase)
    : null;

  if (config.format === 'json') {
    reportJson(report, outputPath);
  } else if (config.format === 'html') {
    const dest = outputPath ?? `hotspot-report-${repoName}.html`;
    reportHtml(report, dest);
  } else {
    if (isMulti) {
      const bar = '═'.repeat(54);
      console.log(`\n╔${bar}╗`);
      console.log(`║  📁 ${repoName.padEnd(48)}  ║`);
      console.log(`║  ${repoPath.slice(-50).padEnd(52)}  ║`);
      console.log(`╚${bar}╝`);
    }
    reportTerminal(report);
  }
}

// ── Main ───────────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  const hasExplicitArgs = process.argv.length > 2;
  const needsInteractive = !positionalPath && !hasExplicitArgs && opts.noInteractive !== true;

  let config: RunConfig = {
    inputPath:  positionalPath ?? process.cwd(),
    since:      opts.since,
    pathFilter: opts.path ?? null,
    topN:       parseInt(opts.top, 10),
    format:     opts.format,
    outputBase: opts.output ?? null,
    bugsOnly:   opts.bugsOnly,
    weights: {
      churn:         parseFloat(opts.weightChurn),
      bugs:          parseFloat(opts.weightBugs),
      reverts:       parseFloat(opts.weightReverts),
      bursts:        parseFloat(opts.weightBursts),
      coupling:      parseFloat(opts.weightCoupling),
      silo:          parseFloat(opts.weightSilo),
      commitQuality: parseFloat(opts.weightCommitQuality),
    },
  };

  // Start ZORP animation for terminal format — runs until analysis begins
  let anim: ZorpHandle | null = null;

  if (needsInteractive) {
    config = await runInteractive(config);
  } else if (config.format === 'terminal') {
    anim = startZorpAnimation();
  }

  // ── Discover repos ───────────────────────────────────────────────────────────
  const repos = findGitRepos(config.inputPath);

  if (repos.length === 0) {
    if (anim) await anim.stop(); // clear ZORP before error message
    console.error(`Error: No git repositories found under: ${config.inputPath}`);
    process.exit(1);
  }

  // Freeze ZORP in place — it stays as a header while output appears below
  if (anim) { await anim.freeze(); anim = null; }

  const isMulti = repos.length > 1;
  if (isMulti) {
    console.error(`\n🔍 Found ${repos.length} git repositories:`);
    for (const r of repos) console.error(`   • ${r}`);
    console.error('');
  }

  // Resolve default output base for html when multi-repo
  if (config.format === 'html' && !config.outputBase) {
    config.outputBase = 'hotspot-report.html';
  }

  const spinner = ora({ text: '', stream: process.stderr }).start();

  for (const repoPath of repos) {
    if (isMulti) {
      spinner.info(`Analyzing: ${basename(repoPath)} (${repoPath})`);
      spinner.start();
    }
    try {
      await runAnalysis(repoPath, config, spinner, isMulti);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      spinner.fail(`${basename(repoPath)}: ${msg}`);
      if (process.env.DEBUG) console.error(err);
    }
  }

  // Print ZORP as a footer after all report output
  if (config.format === 'terminal') {
    printZorpFooter();
  }
}

// ── Interactive setup ──────────────────────────────────────────────────────────

async function runInteractive(defaults: RunConfig): Promise<RunConfig> {
  // ZORP is the welcome screen; stop() waits MIN_DISPLAY_MS then clears.
  await startZorpAnimation().stop();

  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const ask = (q: string): Promise<string> => new Promise(resolve => rl.question(q, resolve));

  console.log('  Interactive Setup');
  console.log('  Accepts a single repo OR a parent folder containing multiple repos.');
  console.log('  Press Enter to accept [defaults], or type a new value.\n');

  let inputPath = '';
  while (true) {
    const raw = await ask(`  Path (drag a folder here or type a path) [${defaults.inputPath}]: `);
    const candidate = raw.trim().replace(/^["']|["']$/g, '') || defaults.inputPath;

    if (!existsSync(candidate)) {
      console.log(`  ⚠  Path not found: ${candidate}`);
      continue;
    }
    const repos = findGitRepos(candidate);
    if (repos.length === 0) {
      console.log(`  ⚠  No git repositories found under: ${candidate}`);
      continue;
    }
    if (repos.length === 1) {
      console.log(`  ✓  Found 1 git repository: ${repos[0]}`);
    } else {
      console.log(`  ✓  Found ${repos.length} git repositories:`);
      for (const r of repos) console.log(`       • ${r}`);
    }
    inputPath = candidate;
    break;
  }

  const sinceDisplay = defaults.since || 'all history';
  const since  = await ask(`  Analyze since [${sinceDisplay}]: `);
  const format = await ask(`  Output format [${defaults.format}] (terminal/json/html): `);
  const fmt    = format.trim() || defaults.format;

  let outputBase = defaults.outputBase;
  if (fmt !== 'terminal') {
    const repos = findGitRepos(inputPath);
    const isMulti = repos.length > 1;
    const defOut = fmt === 'html'
      ? (isMulti ? 'hotspot-report-<repo>.html (one per repo)' : 'hotspot-report.html')
      : (isMulti ? 'hotspot-report-<repo>.json (one per repo)' : 'hotspot-report.json');
    const out = await ask(`  Output base path [${defOut}]: `);
    if (out.trim() && !out.includes('<repo>')) {
      outputBase = out.trim().replace(/^["']|["']$/g, '');
    } else if (!out.trim()) {
      outputBase = fmt === 'html' ? 'hotspot-report.html' : 'hotspot-report.json';
    }
  }

  const top       = await ask(`  Top N results to show in report (all files are always scanned) [${defaults.topN}]: `);
  const bugsOnly  = await ask(`  Bugs-only mode [${defaults.bugsOnly ? 'yes' : 'no'}] (yes/no): `);
  const pathFilt  = await ask(`  Restrict to subdirectory [none]: `);
  const customize = await ask(`  Customize scoring weights? [no] (yes/no): `);

  let weights = { ...defaults.weights };
  if (/^(y|yes)$/i.test(customize.trim())) {
    const pf = (v: string, d: number) => { const n = parseFloat(v.trim()); return isNaN(n) ? d : n; };
    weights = {
      churn:         pf(await ask(`    churn          [${defaults.weights.churn}]: `),         defaults.weights.churn),
      bugs:          pf(await ask(`    bugs           [${defaults.weights.bugs}]: `),          defaults.weights.bugs),
      reverts:       pf(await ask(`    reverts        [${defaults.weights.reverts}]: `),       defaults.weights.reverts),
      bursts:        pf(await ask(`    bursts         [${defaults.weights.bursts}]: `),        defaults.weights.bursts),
      coupling:      pf(await ask(`    coupling       [${defaults.weights.coupling}]: `),      defaults.weights.coupling),
      silo:          pf(await ask(`    silo           [${defaults.weights.silo}]: `),          defaults.weights.silo),
      commitQuality: pf(await ask(`    commit-quality [${defaults.weights.commitQuality}]: `), defaults.weights.commitQuality),
    };
  }

  rl.close();
  console.log('');

  const pi = (v: string, d: number) => { const n = parseInt(v.trim(), 10); return isNaN(n) ? d : n; };

  return {
    inputPath,
    since:      (/^(all|all history)$/i.test(since.trim()) ? '' : since.trim()) || defaults.since,
    format:     fmt,
    outputBase,
    topN:       pi(top, defaults.topN),
    bugsOnly:   bugsOnly.trim() ? /^(y|yes|true)$/i.test(bugsOnly.trim()) : defaults.bugsOnly,
    pathFilter: pathFilt.trim() || null,
    weights,
  };
}

main();

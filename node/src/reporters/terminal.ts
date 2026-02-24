import chalk, { type ChalkInstance } from 'chalk';
import Table from 'cli-table3';
import type { Report, HotspotResult, Tier, SecurityRisk } from '../types.js';

const TIER_LABEL: Record<Tier, string> = {
  CRITICAL: chalk.red('🔴 CRITICAL'),
  HIGH:     chalk.yellow('🟠 HIGH'),
  MEDIUM:   chalk.white('🟡 MEDIUM'),
  LOW:      chalk.green('🟢 LOW'),
};

const RISK_LABEL: Record<SecurityRisk['riskType'], string> = {
  'env-file':        chalk.red('env file'),
  'key-or-cert':     chalk.red('key/cert'),
  'credential-file': chalk.red('credentials'),
};

export function reportTerminal({ meta, results, couplings, securityRisks }: Report): void {
  console.log('');
  console.log(
    chalk.bold.red('🔥 git-scanline') +
    chalk.gray(` — since "${meta.since}"`) +
    chalk.gray(` (${meta.commitCount} commits, ${meta.fileCount} files)`)
  );
  console.log('');

  // ── Security warnings ──────────────────────────────────────────────────────
  if (securityRisks.length > 0) {
    console.log(chalk.red.bold('🔐 Security Risks — sensitive files found in git history:'));
    console.log(chalk.red('   Even deleted files remain accessible via git history!'));
    console.log('');
    for (const risk of securityRisks) {
      console.log(
        `   ${chalk.red('⚠')}  ${chalk.cyan(risk.file)}` +
        `  ${chalk.gray('[')}${RISK_LABEL[risk.riskType]}${chalk.gray(']')}` +
        chalk.gray(`  ${risk.commitCount} commit${risk.commitCount !== 1 ? 's' : ''}`) +
        chalk.gray(`  (first: ${risk.firstSeen}, last: ${risk.lastSeen})`)
      );
    }
    console.log('');
  }

  if (results.length === 0) {
    console.log(chalk.yellow('  No hotspots found with current filters.'));
    console.log('');
    return;
  }

  const table = new Table({
    head: [
      chalk.bold.gray('RANK'),
      chalk.bold.gray('FILE'),
      chalk.bold.gray('SCORE'),
      chalk.bold.gray('CHURN'),
      chalk.bold.gray('BUGS'),
      chalk.bold.gray('REVERTS'),
      chalk.bold.gray('WIP'),
      chalk.bold.gray('RISK'),
    ],
    colWidths: [6, 46, 7, 8, 7, 9, 6, 15],
    style: { head: [], border: ['gray'] },
    chars: {
      top: '─', 'top-mid': '┬', 'top-left': '┌', 'top-right': '┐',
      bottom: '─', 'bottom-mid': '┴', 'bottom-left': '└', 'bottom-right': '┘',
      left: '│', 'left-mid': '├', mid: '─', 'mid-mid': '┼',
      right: '│', 'right-mid': '┤', middle: '│',
    },
  });

  for (let i = 0; i < results.length; i++) {
    const r = results[i]!;
    const scoreColor = scoreChalk(r.hotspotScore);
    const wipStr = r.details.wipCommits > 0
      ? chalk.yellow(String(r.details.wipCommits))
      : chalk.gray('0');

    table.push([
      chalk.gray(String(i + 1).padStart(3)),
      truncatePath(r.file, 44),
      scoreColor(String(r.hotspotScore).padStart(3)),
      makeBar(r.churnScore),
      String(r.details.bugCommits),
      String(r.details.revertCount),
      wipStr,
      TIER_LABEL[r.tier],
    ]);
  }

  console.log(table.toString());

  // ── Co-change coupling ─────────────────────────────────────────────────────
  const notable = couplings.filter(c => c.coChanges >= 5);
  if (notable.length > 0) {
    console.log('');
    console.log(chalk.yellow('⚠️  Co-change coupling detected:'));
    for (const c of notable.slice(0, 5)) {
      console.log(
        `    ${chalk.cyan(c.fileA)} ↔ ${chalk.cyan(c.fileB)} ` +
        chalk.gray(`(changed together ${c.coChanges}x, strength ${c.strength}%)`)
      );
    }
  }

  // ── Recommendations ────────────────────────────────────────────────────────
  const recs = buildRecommendations(results);
  if (recs.length > 0) {
    console.log('');
    console.log(chalk.cyan('💡 Recommendations:'));
    for (const rec of recs) {
      console.log(`    ${chalk.white('•')} ${rec}`);
    }
  }

  console.log('');
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeBar(score: number): string {
  const FULL  = '█';
  const PARTS = ['', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
  const filled  = Math.floor(score / 20);
  const rem     = score % 20;
  const partial = PARTS[Math.round(rem / 2.5)] ?? '';
  return chalk.red((FULL.repeat(filled) + partial).padEnd(5));
}

function scoreChalk(score: number): ChalkInstance {
  if (score >= 75) return chalk.red.bold;
  if (score >= 50) return chalk.yellow.bold;
  if (score >= 25) return chalk.white;
  return chalk.green;
}

function truncatePath(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return chalk.gray('…') + str.slice(-(maxLen - 1));
}

function buildRecommendations(results: HotspotResult[]): string[] {
  const recs: string[] = [];

  for (const r of results.slice(0, 10)) {
    const name = r.file.split('/').pop() ?? r.file;

    if (r.details.topAuthorPercent >= 80 && r.details.authorCount <= 2) {
      recs.push(
        `${chalk.yellow(name)} has ${chalk.bold(r.details.topAuthorPercent + '%')} ` +
        `single-author commits — consider a knowledge-transfer session`
      );
    }
    if (r.details.burstIncidents >= 3) {
      recs.push(
        `${chalk.yellow(name)} shows burst patterns: ` +
        `${chalk.bold(String(r.details.burstIncidents))} rapid-commit windows detected`
      );
    }
    if (r.details.revertCount >= 2) {
      recs.push(
        `${chalk.yellow(name)} has been reverted ${chalk.bold(String(r.details.revertCount))} times` +
        ` — consider adding tests or stricter review`
      );
    }
    if (r.details.wipCommits >= 3) {
      recs.push(
        `${chalk.yellow(name)} appears in ${chalk.bold(String(r.details.wipCommits))} WIP/low-quality commits` +
        ` — this area may need more careful review practices`
      );
    }
    if (r.details.largeCommitCount >= 3) {
      recs.push(
        `${chalk.yellow(name)} was swept up in ${chalk.bold(String(r.details.largeCommitCount))} large commits` +
        ` — consider smaller, focused PRs`
      );
    }
  }

  return recs.slice(0, 8);
}

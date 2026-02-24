import { writeFileSync } from 'fs';
import type { Report, HotspotResult, CouplingEntry, ReportMeta, Tier, SecurityRisk } from '../types.js';

export function reportHtml(report: Report, outputFile: string = 'hotspot-report.html'): void {
  const html = buildHtml(report.meta, report.results, report.couplings, report.securityRisks);
  writeFileSync(outputFile, html, 'utf8');
  console.error(`✓ HTML report written to ${outputFile}`);
}

function buildHtml(
  meta: ReportMeta,
  results: HotspotResult[],
  couplings: CouplingEntry[],
  securityRisks: SecurityRisk[]
): string {
  const top = results.slice(0, 25);

  const chartLabels = JSON.stringify(top.map(r => r.file.split('/').slice(-2).join('/')));
  const chartScores = JSON.stringify(top.map(r => r.hotspotScore));
  const chartColors = JSON.stringify(top.map(r => tierColor(r.tier)));
  const fullLabels  = JSON.stringify(top.map(r => r.file));

  const critCount       = results.filter(r => r.tier === 'CRITICAL').length;
  const highCount       = results.filter(r => r.tier === 'HIGH').length;
  const totalBugCommits = results.reduce((s, r) => s + r.details.bugCommits, 0);

  const tableRows = results.map((r, i) => `
    <tr>
      <td class="num">${i + 1}</td>
      <td class="path">${esc(r.file)}</td>
      <td class="num"><strong>${r.hotspotScore}</strong></td>
      <td class="num">${r.details.commitCount}</td>
      <td class="num">${r.details.bugCommits}</td>
      <td class="num">${r.details.revertCount}</td>
      <td class="num">${r.details.wipCommits > 0 ? `<span class="warn">${r.details.wipCommits}</span>` : '0'}</td>
      <td class="num">${r.details.largeCommitCount > 0 ? `<span class="dim">${r.details.largeCommitCount}</span>` : '0'}</td>
      <td>${esc(r.details.topAuthor)} <span class="dim">(${r.details.topAuthorPercent}%)</span></td>
      <td>${tierBadge(r.tier)}</td>
    </tr>`).join('');

  const couplingRows = couplings.slice(0, 15).map(c => `
    <tr>
      <td class="path">${esc(c.fileA)}</td>
      <td class="path">${esc(c.fileB)}</td>
      <td class="num">${c.coChanges}</td>
      <td class="num">${c.strength}%</td>
    </tr>`).join('');

  const securitySection = securityRisks.length > 0 ? `
  <div class="card security-card">
    <h2>🔐 Security Risks</h2>
    <p class="security-note">Sensitive files found in git history. Even deleted files remain accessible via <code>git log</code>.</p>
    <table>
      <thead><tr><th>File</th><th>Risk Type</th><th style="text-align:right">Commits</th><th>First Seen</th><th>Last Seen</th></tr></thead>
      <tbody>
        ${securityRisks.map(r => `
        <tr>
          <td class="path">${esc(r.file)}</td>
          <td><span class="badge badge-critical">${esc(r.riskType)}</span></td>
          <td class="num">${r.commitCount}</td>
          <td class="num">${esc(r.firstSeen)}</td>
          <td class="num">${esc(r.lastSeen)}</td>
        </tr>`).join('')}
      </tbody>
    </table>
  </div>` : '';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>git-scanline report</title>
  <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #0f172a; color: #e2e8f0; padding: 2rem; font-size: 14px; line-height: 1.6;
    }
    h1 { font-size: 1.75rem; font-weight: 800; }
    h1 .fire { color: #f97316; }
    h2 { font-size: 1rem; font-weight: 700; color: #94a3b8; text-transform: uppercase; letter-spacing: .06em; margin-bottom: 1rem; }
    .meta { color: #64748b; margin: .4rem 0 2rem; font-size: .8rem; }
    .meta span { margin-right: 1.5rem; }
    .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 1rem; margin-bottom: 1.5rem; }
    .stat { background: #1e293b; border: 1px solid #334155; border-radius: .625rem; padding: 1rem 1.25rem; }
    .stat-label { font-size: .7rem; text-transform: uppercase; letter-spacing: .06em; color: #64748b; }
    .stat-value { font-size: 2rem; font-weight: 800; margin-top: .2rem; }
    .stat-value.red    { color: #f87171; }
    .stat-value.orange { color: #fb923c; }
    .stat-value.blue   { color: #60a5fa; }
    .stat-value.slate  { color: #94a3b8; }
    .card { background: #1e293b; border: 1px solid #334155; border-radius: .75rem; padding: 1.5rem; margin-bottom: 1.5rem; }
    .security-card { border-color: #ef4444; }
    .security-note { color: #fca5a5; font-size: .8rem; margin-bottom: 1rem; }
    .security-note code { background: #1a0e0e; padding: .1rem .3rem; border-radius: .2rem; font-size: .75rem; }
    .chart-wrap { position: relative; height: 280px; }
    table { width: 100%; border-collapse: collapse; }
    th { text-align: left; padding: .5rem .75rem; border-bottom: 2px solid #334155; font-size: .7rem; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: #64748b; }
    td { padding: .5rem .75rem; border-bottom: 1px solid #1a2744; vertical-align: middle; }
    tr:last-child td { border-bottom: none; }
    tr:hover td { background: #162032; }
    td.path { font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace; font-size: .78rem; color: #7dd3fc; word-break: break-all; }
    td.num { text-align: right; color: #94a3b8; }
    .dim { color: #475569; font-size: .8em; }
    .warn { color: #fbbf24; font-weight: 600; }
    .badge { display: inline-block; padding: .15rem .5rem; border-radius: 999px; font-size: .7rem; font-weight: 700; white-space: nowrap; }
    .badge-critical { background: rgba(239,68,68,.15);  color: #fca5a5; }
    .badge-high     { background: rgba(249,115,22,.15); color: #fdba74; }
    .badge-medium   { background: rgba(234,179,8,.15);  color: #fde047; }
    .badge-low      { background: rgba(34,197,94,.15);  color: #86efac; }
    .footer { text-align: center; color: #334155; font-size: .75rem; margin-top: 2rem; }
  </style>
</head>
<body>
  <h1><span class="fire">🔥</span> git-scanline</h1>
  <p class="meta">
    <span>Since: <strong>${esc(meta.since)}</strong></span>
    <span>Generated: <strong>${new Date(meta.analyzedAt).toLocaleString()}</strong></span>
    <span>Commits: <strong>${meta.commitCount.toLocaleString()}</strong></span>
    <span>Files scanned: <strong>${meta.fileCount.toLocaleString()}</strong></span>
  </p>

  <div class="stats">
    <div class="stat"><div class="stat-label">Critical Hotspots</div><div class="stat-value red">${critCount}</div></div>
    <div class="stat"><div class="stat-label">High Risk Files</div><div class="stat-value orange">${highCount}</div></div>
    <div class="stat"><div class="stat-label">Total Commits</div><div class="stat-value blue">${meta.commitCount.toLocaleString()}</div></div>
    <div class="stat"><div class="stat-label">Bug-fix Commits</div><div class="stat-value slate">${totalBugCommits.toLocaleString()}</div></div>
    ${securityRisks.length > 0 ? `<div class="stat" style="border-color:#ef4444"><div class="stat-label" style="color:#f87171">Security Risks</div><div class="stat-value red">${securityRisks.length}</div></div>` : ''}
  </div>

  ${securitySection}

  <div class="card">
    <h2>Top Hotspot Files — Score (0–100)</h2>
    <div class="chart-wrap"><canvas id="chart"></canvas></div>
  </div>

  <div class="card">
    <h2>Hotspot Details</h2>
    <table>
      <thead>
        <tr>
          <th>#</th><th>File</th><th style="text-align:right">Score</th><th style="text-align:right">Commits</th>
          <th style="text-align:right">Bug Commits</th><th style="text-align:right">Reverts</th>
          <th style="text-align:right">WIP</th><th style="text-align:right">Large</th>
          <th>Top Author</th><th>Risk</th>
        </tr>
      </thead>
      <tbody>${tableRows}</tbody>
    </table>
  </div>

  ${couplings.length > 0 ? `
  <div class="card">
    <h2>⚠️ Co-change Coupling</h2>
    <p style="color:#64748b;font-size:.8rem;margin-bottom:1rem">Files that frequently change together — hidden dependencies that lead to bugs.</p>
    <table>
      <thead><tr><th>File A</th><th>File B</th><th style="text-align:right">Co-changes</th><th style="text-align:right">Coupling Strength</th></tr></thead>
      <tbody>${couplingRows}</tbody>
    </table>
  </div>` : ''}

  <p class="footer">Generated by git-scanline on ${new Date().toLocaleString()}</p>

  <script>
    const ctx = document.getElementById('chart');
    new Chart(ctx, {
      type: 'bar',
      data: {
        labels: ${chartLabels},
        datasets: [{ label: 'Hotspot Score', data: ${chartScores}, backgroundColor: ${chartColors}, borderRadius: 4, borderSkipped: false }]
      },
      options: {
        responsive: true, maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: { callbacks: { title: items => ${fullLabels}[items[0].dataIndex], label: items => ' Score: ' + items.raw } }
        },
        scales: {
          y: { beginAtZero: true, max: 100, ticks: { color: '#64748b' }, grid: { color: '#1e2d47' } },
          x: { ticks: { color: '#64748b', maxRotation: 40, font: { size: 11 } }, grid: { display: false } }
        }
      }
    });
  </script>
</body>
</html>`;
}

function esc(str: string): string {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

function tierColor(tier: Tier): string {
  switch (tier) {
    case 'CRITICAL': return 'rgba(239,68,68,0.75)';
    case 'HIGH':     return 'rgba(249,115,22,0.75)';
    case 'MEDIUM':   return 'rgba(234,179,8,0.75)';
    default:         return 'rgba(34,197,94,0.75)';
  }
}

function tierBadge(tier: Tier): string {
  switch (tier) {
    case 'CRITICAL': return '<span class="badge badge-critical">🔴 CRITICAL</span>';
    case 'HIGH':     return '<span class="badge badge-high">🟠 HIGH</span>';
    case 'MEDIUM':   return '<span class="badge badge-medium">🟡 MEDIUM</span>';
    default:         return '<span class="badge badge-low">🟢 LOW</span>';
  }
}

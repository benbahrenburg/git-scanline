import { writeFileSync } from 'fs';
import type { Report } from '../types.js';

/**
 * Outputs the report as machine-readable JSON.
 * If outputFile is provided, writes to disk; otherwise writes to stdout.
 */
export function reportJson(report: Report, outputFile: string | null = null): void {
  const payload = JSON.stringify(report, null, 2);

  if (outputFile) {
    writeFileSync(outputFile, payload, 'utf8');
    console.error(`✓ JSON report written to ${outputFile}`);
  } else {
    process.stdout.write(payload + '\n');
  }
}

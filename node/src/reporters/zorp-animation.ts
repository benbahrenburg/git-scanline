import chalk from 'chalk';

// ── Wave ───────────────────────────────────────────────────────────────────────

const WAVE_BASE = '~^~^~~';
const WAVE_LEN  = 54;

function waveStr(frame: number): string {
  const offset = frame % WAVE_BASE.length;
  const long   = WAVE_BASE.repeat(20);
  const window = long.slice(offset, offset + WAVE_LEN);
  return '   ' + window;
}

// ── Frame builder ──────────────────────────────────────────────────────────────

const LINE_COUNT = 7;

function buildFrame(frame: number): string[] {
  const variantB = frame % 8 >= 4;
  const antenna  = variantB
    ? chalk.cyan('      |\\  /\\     ')
    : chalk.cyan('      |\\  /|      ');
  return [
    antenna,
    chalk.white.bold('     (o \\/ o)     '),
    chalk.cyan('      |====|      '),
    chalk.cyan('     /| || |\\     '),
    chalk.cyan('    / |_||_| \\    '),
    chalk.blue.bold('   /___________\\  '),
    chalk.cyan(waveStr(frame)),
  ];
}

// ── Rendering ─────────────────────────────────────────────────────────────────

function writeFrame(frame: number, first: boolean): void {
  if (!first) {
    process.stderr.write(`\x1b[${LINE_COUNT}A`);
  }
  for (const line of buildFrame(frame)) {
    process.stderr.write(`\x1b[2K\r${line}\n`);
  }
}

const sleep = (ms: number) => new Promise<void>(r => setTimeout(r, ms));

// ── Public API ─────────────────────────────────────────────────────────────────

const MIN_DISPLAY_MS = 1600;

export interface ZorpHandle {
  /**
   * Waits for `MIN_DISPLAY_MS`, stops the loop, **erases** the animation area,
   * and restores the cursor. Use in error paths.
   */
  stop(): Promise<void>;

  /**
   * Waits for `MIN_DISPLAY_MS`, stops the loop, but **leaves the last ZORP
   * frame visible** on screen. Cursor is positioned on the line below ZORP —
   * ready for spinner or report output to appear below it.
   */
  freeze(): Promise<void>;
}

/**
 * Starts the ZORP surfing animation on stderr and returns a {@link ZorpHandle}.
 * No-op when stderr is not a TTY.
 */
export function startZorpAnimation(): ZorpHandle {
  if (!process.stderr.isTTY) {
    return { stop: async () => {}, freeze: async () => {} };
  }

  let running   = true;
  let doClear   = true;  // freeze() sets this to false before stopping
  const startMs = Date.now();

  let resolveStop: () => void = () => {};
  const stopped = new Promise<void>(r => { resolveStop = r; });

  const cleanup = () => {
    process.stderr.write('\x1b[?25h');
    process.exit(130);
  };
  process.once('SIGINT', cleanup);
  process.stderr.write('\x1b[?25l');

  (async () => {
    let frame = 0;
    while (running) {
      writeFrame(frame, frame === 0);
      frame++;
      await sleep(200);
    }
    if (doClear && frame > 0) {
      // stop() path: erase animation area
      process.stderr.write(`\x1b[${LINE_COUNT}A\x1b[0J`);
    }
    // freeze() path: leave ZORP on screen; cursor already below last frame
    process.stderr.write('\x1b[?25h');
    process.removeListener('SIGINT', cleanup);
    resolveStop();
  })();

  const waitMin = async () => {
    const elapsed = Date.now() - startMs;
    if (elapsed < MIN_DISPLAY_MS) await sleep(MIN_DISPLAY_MS - elapsed);
  };

  return {
    async stop() {
      await waitMin();
      // doClear stays true
      running = false;
      await stopped;
    },
    async freeze() {
      await waitMin();
      doClear = false;
      running = false;
      await stopped;
    },
  };
}

/**
 * Prints one static ZORP frame to stderr as a footer after the report.
 * No-op when stderr is not a TTY.
 */
export function printZorpFooter(): void {
  if (!process.stderr.isTTY) return;
  process.stderr.write('\n');
  for (const line of buildFrame(4)) { // frame 4: antenna raised — waving goodbye
    process.stderr.write(`${line}\n`);
  }
  process.stderr.write('\n');
}

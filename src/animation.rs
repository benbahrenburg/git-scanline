use colored::Colorize;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ── Wave ───────────────────────────────────────────────────────────────────────

const WAVE_BASE: &str = "~^~^~~";
const WAVE_LEN: usize = 54;

fn wave_line(frame: usize) -> String {
    let offset = frame % WAVE_BASE.len();
    let long = WAVE_BASE.repeat(20);
    let window = &long[offset..offset + WAVE_LEN];
    format!("   {}", window)
}

// ── Frame builder ──────────────────────────────────────────────────────────────

const LINE_COUNT: usize = 7;

fn build_frame(frame: usize) -> [String; LINE_COUNT] {
    let antenna: String = if frame % 8 < 4 {
        "      |\\  /|      ".cyan().to_string()
    } else {
        "      |\\  /\\     ".cyan().to_string()
    };
    [
        antenna,
        "     (o \\/ o)     ".white().bold().to_string(),
        "      |====|      ".cyan().to_string(),
        "     /| || |\\     ".cyan().to_string(),
        "    / |_||_| \\    ".cyan().to_string(),
        "   /___________\\  ".blue().bold().to_string(),
        wave_line(frame).cyan().to_string(),
    ]
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn write_frame(frame: usize, first: bool) {
    let mut err = std::io::stderr();
    if !first {
        write!(err, "\x1b[{}A", LINE_COUNT).ok();
    }
    for line in build_frame(frame).iter() {
        write!(err, "\x1b[2K\r{}\n", line).ok();
    }
    err.flush().ok();
}

// ── Public API ─────────────────────────────────────────────────────────────────

const MIN_DISPLAY_MS: u64 = 1600;

/// Handle to a running ZORP animation.
///
/// - [`freeze`](ZorpHandle::freeze) — stops the animation loop but leaves the
///   last frame visible on screen; cursor is positioned below ZORP, ready for
///   spinner/report output.
/// - [`stop`](ZorpHandle::stop) — stops and **clears** the animation area.
/// - *Dropping* without calling either method clears immediately (error paths).
pub struct ZorpHandle {
    running: Arc<AtomicBool>,
    clear_on_stop: Arc<AtomicBool>, // true → erase on stop; false → leave in place
    thread: Option<thread::JoinHandle<()>>,
    started_at: Instant,
    is_noop: bool,
}

fn wait_min(started_at: Instant) {
    let elapsed = started_at.elapsed();
    let min = Duration::from_millis(MIN_DISPLAY_MS);
    if elapsed < min {
        thread::sleep(min - elapsed);
    }
}

impl ZorpHandle {
    /// Stops animation, waits for `MIN_DISPLAY_MS`, and **erases** the
    /// terminal area before returning.
    pub fn stop(self) {
        if !self.is_noop {
            wait_min(self.started_at);
        }
        // clear_on_stop remains true — Drop will erase ZORP
    }

    /// Stops animation, waits for `MIN_DISPLAY_MS`, then leaves the last frame
    /// **frozen on screen**. Cursor is positioned on the line below ZORP.
    /// Use this to keep ZORP visible as a header while the spinner runs below.
    pub fn freeze(self) {
        if !self.is_noop {
            wait_min(self.started_at);
            self.clear_on_stop.store(false, Ordering::Relaxed);
        }
        // Drop signals thread to stop; thread skips the clear because clear_on_stop=false
    }
}

impl Drop for ZorpHandle {
    fn drop(&mut self) {
        if self.is_noop {
            return;
        }
        self.running.store(false, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().ok();
        }
    }
}

/// Starts the ZORP surfing animation on stderr in a background thread.
/// No-op when stderr is not a TTY.
pub fn start_zorp() -> ZorpHandle {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return ZorpHandle {
            running: Arc::new(AtomicBool::new(false)),
            clear_on_stop: Arc::new(AtomicBool::new(true)),
            thread: None,
            started_at: Instant::now(),
            is_noop: true,
        };
    }

    let running = Arc::new(AtomicBool::new(true));
    let clear_on_stop = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let c = clear_on_stop.clone();

    let handle = thread::spawn(move || {
        {
            let mut err = std::io::stderr();
            write!(err, "\x1b[?25l").ok();
            err.flush().ok();
        }

        let mut frame = 0usize;
        loop {
            if !r.load(Ordering::Relaxed) {
                break;
            }
            write_frame(frame, frame == 0);
            frame += 1;
            thread::sleep(Duration::from_millis(200));
        }

        let mut err = std::io::stderr();
        if c.load(Ordering::Relaxed) {
            // stop() path: erase animation and restore cursor
            if frame > 0 {
                write!(err, "\x1b[{}A\x1b[0J", LINE_COUNT).ok();
            }
        }
        // freeze() path: leave ZORP on screen; cursor is already below last frame
        write!(err, "\x1b[?25h").ok();
        err.flush().ok();
    });

    ZorpHandle {
        running,
        clear_on_stop,
        thread: Some(handle),
        started_at: Instant::now(),
        is_noop: false,
    }
}

/// Prints one static ZORP frame to stderr as a footer after the report.
/// No-op when stderr is not a TTY.
pub fn print_zorp_footer() {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return;
    }
    let mut err = std::io::stderr();
    writeln!(err).ok();
    for line in build_frame(4).iter() {
        // frame 4: antenna raised — waving goodbye
        writeln!(err, "{}", line).ok();
    }
    writeln!(err).ok();
    err.flush().ok();
}

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Per-frame timing instrumentation, gated behind the `UZUMAKI_PERF`
/// environment variable. When unset, `enabled()` is false and callers skip
/// all measurement. The latest frame's timings are kept so an on-screen debug
/// HUD can read them without re-measuring.
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("UZUMAKI_PERF").is_some())
}

#[derive(Clone, Copy)]
pub struct FrameTimings {
    pub styles: Duration,
    pub layout_children: Duration,
    pub layout: Duration,
    pub hit_tree: Duration,
    pub paint: Duration,
    pub node_count: usize,
}

impl FrameTimings {
    pub const ZERO: Self = Self {
        styles: Duration::ZERO,
        layout_children: Duration::ZERO,
        layout: Duration::ZERO,
        hit_tree: Duration::ZERO,
        paint: Duration::ZERO,
        node_count: 0,
    };

    pub fn total(&self) -> Duration {
        self.styles + self.layout_children + self.layout + self.hit_tree + self.paint
    }
}

impl Default for FrameTimings {
    fn default() -> Self {
        Self::ZERO
    }
}

static LATEST: Mutex<FrameTimings> = Mutex::new(FrameTimings::ZERO);

/// Store the layout-pass timings for this frame (paint is recorded separately,
/// after the scene is built).
pub fn record_layout_passes(timings: FrameTimings) {
    if let Ok(mut latest) = LATEST.lock() {
        *latest = timings;
    }
}

/// Fill in the paint duration for the current frame and emit a throttled log.
pub fn record_paint(paint: Duration) {
    let timings = {
        let Ok(mut latest) = LATEST.lock() else {
            return;
        };
        latest.paint = paint;
        *latest
    };
    log_throttled(&timings);
}

/// Latest recorded frame timings. Intended for the debug HUD.
pub fn latest() -> FrameTimings {
    LATEST.lock().map(|t| *t).unwrap_or_default()
}

fn log_throttled(t: &FrameTimings) {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let Ok(mut last) = LAST.lock() else {
        return;
    };
    let now = Instant::now();
    if last.is_some_and(|l| now.duration_since(l) < Duration::from_millis(500)) {
        return;
    }
    *last = Some(now);
    eprintln!(
        "[perf] nodes={} styles={:.2} children={:.2} layout={:.2} hit={:.2} paint={:.2} total={:.2} (ms)",
        t.node_count,
        ms(t.styles),
        ms(t.layout_children),
        ms(t.layout),
        ms(t.hit_tree),
        ms(t.paint),
        ms(t.total()),
    );
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

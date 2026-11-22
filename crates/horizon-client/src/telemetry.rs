//! What is left of Horizon's opt-in data collection: the start-up marks and the adapter's name.
//! Nothing is kept or sent.
use std::sync::OnceLock;
use std::time::Instant;

static STARTED: OnceLock<Instant> = OnceLock::new();

/// When the game started, for the time to the title screen. Call first thing in `main`.
pub fn mark_start() {
    STARTED.get_or_init(Instant::now);
}

/// The adapter's name, once the renderer has one.
pub fn gpu_name() -> Option<String> {
    crate::gfx::adapter().map(|gpu| gpu.name)
}

/// The release (`v0.2.3`), or the client's crate version for a build from source.
pub fn version() -> String {
    crate::crash::release_tag().unwrap_or_else(|| format!("dev {}", env!("CARGO_PKG_VERSION")))
}

/// One play session, for the start-up spans.
#[derive(Default)]
pub struct Session {
    title_measured: bool,
    opening: Option<Instant>,
}

impl Session {
    /// The title screen drew a frame: the first time, the start-up span ends.
    pub fn title_shown(&mut self) {
        if !self.title_measured {
            self.title_measured = true;
            crate::spans::startup::mark("title.shown");
        }
    }

    /// A world starts loading.
    pub fn world_opening(&mut self) {
        if self.opening.is_none() {
            crate::spans::startup::arm("world.opening");
        }
        self.opening.get_or_insert_with(Instant::now);
    }

    /// An entry queued during startup was cancelled before opening a world.
    pub fn cancel_world_opening(&mut self) {
        self.opening = None;
    }
}

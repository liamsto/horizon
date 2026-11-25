//! What `horizon-server` asks the dedicated server on its control endpoint (`--control-socket`, docs/server.md
//! section 2): `server-status` answers with the tick rate, the tick times `/tick query` reports, the players
//! (including their positions) and the loaded chunks; `server-mods` answers with the canonical loaded package
//! inventory; `console <line>` runs a console command and answers with the lines it logged;
//! `server-events <since>` answers the typed events after that number (events.rs); `management <connection>
//! <json>` edits the lists as vanilla's management protocol does (management.rs).
//! `player-expect <uuid> [seconds]`, `player-drop <uuid>`, `player-export <uuid> [freeze]`,
//! `player-release <uuid> ...`, `player-import <uuid> <json>` and `player-confirm <uuid> ...` are a proxy's,
//! for a player it moves between two servers, and `transfer-status` the counts it reads first (transfer.rs).
use super::ServerState;
use crate::game::Game;
use std::time::{Duration, Instant};

/// The window the tick rate is measured over.
const TPS_WINDOW: Duration = Duration::from_secs(5);

impl ServerState {
    /// One pass of the server loop ended.
    pub(super) fn ticked(&mut self, at: Instant) {
        self.ticks.push_back(at);
        while self
            .ticks
            .front()
            .is_some_and(|t| at.duration_since(*t) > TPS_WINDOW)
        {
            self.ticks.pop_front();
        }
    }
    /// Ticks per second over the last few seconds.
    pub(super) fn tps(&self) -> f64 {
        match (self.ticks.front(), self.ticks.back()) {
            (Some(first), Some(last)) if self.ticks.len() > 1 => {
                (self.ticks.len() - 1) as f64 / last.duration_since(*first).as_secs_f64().max(1e-9)
            }
            _ => 0.0,
        }
    }
}

impl Game<'_> {

}

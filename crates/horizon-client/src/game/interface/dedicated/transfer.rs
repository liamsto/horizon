//! A proxy's requests on the control endpoint, for a player it moves between two servers:
//! `player-expect <uuid> [seconds]` (the target is told who is coming; its login is held),
//! `player-drop <uuid>` (the target forgets it, and lets a held player go),
//! `player-export <uuid> [freeze]` (the source answers with the player's record) and
//! `player-release <uuid> <hand-over> moved [announce] | abort` (the source lets the player go),
//! `player-import <uuid> <json>` and `player-confirm <uuid> <hand-over>` (the target takes the
//! record and lets the held player in), and `transfer-status` (the counts a proxy reads before
//! it sends players here). They run
//! between two ticks, on the server thread, so the player is never caught half-way through a
//! tick. The rules are `lan/arrivals.rs`'s. All of them are refused while `server.toml`'s
//! `[transfer]` is off, and those that bring a player in once the server has begun to stop.
use crate::game::Game;
use horizon_client::lan::arrivals;
use std::time::Instant;

impl Game<'_> {

    /// Whether the tick loop ends now. A stop (`/stop`, a stop signal) ends it at once, unless
    /// `[transfer]` is on and players are here: the stop then waits for a proxy to move them out
    /// (`arrivals::StopWait`), every login refused, the server ticking on. A second `stop` ends the
    /// wait; a second signal ends the process (`horizon_threads::catch_stop_signals`).
    pub(super) fn stop_now(&mut self, state: &super::ServerState) -> bool {
        let asked = std::mem::take(&mut state.lists.lock().unwrap().stop);
        let stop = asked || horizon_threads::stop_requests() > 0;
        let Some(host) = self.lan.as_mut() else {
            return stop;
        };
        let waiting = host
            .hub
            .transfers
            .as_ref()
            .is_some_and(|t| t.stop_wait.is_some());
        if !stop && !waiting {
            return false;
        }
        let (placed, frozen) = arrivals::placed_and_frozen(&host.hub);
        let Some(transfers) = host.hub.transfers.as_mut() else {
            return stop;
        };
        let now = Instant::now();
        // A freeze lasts ticks: how long one takes now, for `check_freeze`.
        let period = arrivals::tick_period(self.clock.tick_duration(), state.tps());
        match &mut transfers.stop_wait {
            None => {
                transfers.stopping = true;
                let wait = arrivals::StopWait::start(&transfers.settings, now, period);
                let Some(wait) = wait.filter(|_| placed > 0) else {
                    return true;
                };
                transfers.stop_wait = Some(wait);
                host.hub.lan.shared.refuse_logins();
                super::log(
                    "INFO",
                    &arrivals::stop_wait_line(transfers.settings.stop_wait),
                );
                false
            }
            Some(_) if asked => {
                super::log("INFO", "A second stop: stopping now");
                true
            }
            Some(wait) => {
                wait.period = period;
                let over = wait.over(now, placed, frozen);
                if over && frozen > 0 {
                    super::log(
                        "WARN",
                        &format!(
                            "Stopping with {frozen} hand-over(s) still frozen: their players are saved here"
                        ),
                    );
                }
                over
            }
        }
    }

}

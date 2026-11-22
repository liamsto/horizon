//! Who caused an input event: a person at the keyboard (`user`), an agent on the
//! control socket (`agent`), or the game itself (`game`, made-up input no one
//! asked for). Agents read this with `input-log` and the `input` section of
//! `state` and `status`, so a script can tell that a person took over instead of
//! mistaking their keystrokes for its own. A `user-input` event also lands in
//! `events` (and so in `wait-for event user-input`) when a person steps in
//! while an agent was driving.
//!
//! The events are also what a repro recording writes to `inputs.jsonl`
//! (game/runtime/repro.rs).
use super::{Game, Screen};
use horizon_client::keybinds;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

/// Input events kept for `input-log`.
const KEPT: usize = 512;
/// A person's input this soon after an agent command is an intrusion.
const INTRUSION_GAP: Duration = Duration::from_secs(5);
/// At most one `user-input` event per this long, so holding a key does not flood.
const INTRUSION_QUIET: Duration = Duration::from_secs(2);

/// Who caused an input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum InputOrigin {
    /// The window's own events: someone at this computer's keyboard, mouse or pad.
    User,
    /// The control socket: `horizon-ctl`, an MCP client, or a script.
    Agent,
    /// Neither: input the game made up itself (pad menus, UI tasks).
    Game,
}

impl InputOrigin {
    fn index(self) -> usize {
        self as usize
    }
}

/// One input event, as `input-log` shows it and the recording writes it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct TraceEvent {
    pub seq: u64,
    pub tick: u64,
    /// Milliseconds since the first event this run; the clock a replay keeps.
    pub ms: u64,
    pub origin: InputOrigin,
    #[serde(flatten)]
    pub kind: TraceKind,
}

/// What was pressed, clicked or typed. Key and button names are the game's own
/// (`key.keyboard.w`, `key.mouse.left`), so a recording can feed them back.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum TraceKind {
    Key {
        key: String,
        /// `down`, `up` or `repeat` (the system's auto-repeat).
        action: String,
    },
    Mouse {
        button: String,
        action: String,
    },
    Wheel {
        x: f64,
        y: f64,
        /// The window gave lines (a notched wheel) rather than pixels.
        lines: bool,
    },
    /// The pointer's screen position, kept while a screen is open: GUI clicks
    /// are replayed at it.
    Cursor {
        x: f64,
        y: f64,
    },
    /// Relative mouse motion, which turns the view.
    Motion {
        x: f64,
        y: f64,
    },
    Pad {
        button: String,
        action: String,
    },
    /// A control-socket line: `key`, `mouse`, `goto`, anything an agent ran.
    Command {
        line: String,
    },
}

impl TraceKind {
}

/// The recent input, the counters and the recording's line writer.
#[derive(Default)]
pub(super) struct InputTrace {
    seq: u64,
    log: VecDeque<TraceEvent>,
    counts: [u64; 3],
    last: Option<InputOrigin>,
    last_user: Option<Instant>,
    last_agent: Option<Instant>,
    /// The person's events since the last agent command.
    user_since_agent: u64,
    quiet_until: Option<Instant>,
    /// The first event's wall clock and tick.
    started: Option<(Instant, u64)>,
    /// The recording's `inputs.jsonl` writer; `None` when nothing records.
    writer: Option<Sender<String>>,
    written: u64,
    /// A replay feeds events back: they are not new input and are not recorded.
    suppress: bool,
}

impl InputTrace {
    /// Record one event; whether it is a person intruding on an agent's run.
    /// `agent_active`: an agent is watching or waiting, not only recently heard.
    fn record(
        &mut self,
        tick: u64,
        origin: InputOrigin,
        kind: TraceKind,
        agent_active: bool,
    ) -> bool {
        if self.suppress {
            return false;
        }
        let now = Instant::now();
        let started = self.started.get_or_insert((now, tick)).0;
        let ms = now.duration_since(started).as_millis() as u64;

        let intrusion = origin == InputOrigin::User
            && (agent_active
                || self
                    .last_agent
                    .is_some_and(|t| now.duration_since(t) < INTRUSION_GAP))
            && self.quiet_until.is_none_or(|t| now >= t);
        if intrusion {
            self.quiet_until = Some(now + INTRUSION_QUIET);
        }

        self.seq += 1;
        let event = TraceEvent {
            seq: self.seq,
            tick,
            ms,
            origin,
            kind,
        };
        if self.writer.is_some() {
            let mut failed = false;
            match serde_json::to_string(&event) {
                Ok(line) => {
                    if let Some(writer) = &self.writer {
                        failed = writer.send(line).is_err();
                    }
                }
                Err(e) => log::warn!("Horizon: input trace: {e}"),
            }
            if failed {
                self.writer = None;
                log::warn!("Horizon: input trace stopped: its writer is gone");
            } else {
                self.written += 1;
            }
        }
        if self.log.len() == KEPT {
            self.log.pop_front();
        }
        self.log.push_back(event);
        self.counts[origin.index()] += 1;
        self.last = Some(origin);
        match origin {
            InputOrigin::User => {
                self.last_user = Some(now);
                self.user_since_agent += 1;
            }
            InputOrigin::Agent => {
                self.last_agent = Some(now);
                self.user_since_agent = 0;
            }
            InputOrigin::Game => {}
        }
        intrusion
    }

}

impl Game<'_> {

    /// Whether anything can read the trace: the control socket (agent control
    /// on), the repro shortcuts, or a recording under way. Otherwise every key
    /// and mouse motion skips it, so a player who uses none of them pays
    /// nothing per input event.
    fn tracing_input(&self) -> bool {
        false
    }

    /// Record an input event and, when a person steps into an agent's run, tell
    /// every watcher once.
    pub(super) fn note_input(&mut self, origin: InputOrigin, kind: TraceKind) {
        if !self.tracing_input() {
            return;
        }
        self.input_trace.record(self.ticks, origin, kind, false);
    }

    /// Keep the window events that are input. Agent-made window events are left
    /// out: the control-socket line that caused them is recorded instead.
    pub(super) fn trace_window_event(&mut self, origin: InputOrigin, event: &WindowEvent) {
        if origin == InputOrigin::Agent || !self.tracing_input() {
            return;
        }
        match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let Some(name) = keybinds::key_name(code) else {
                    return;
                };
                let action = match (event.state, event.repeat) {
                    (ElementState::Released, _) => "up",
                    (_, true) => "repeat",
                    (ElementState::Pressed, false) => "down",
                };
                // The platform made this one up (an IME's dead key): not a person.
                let origin = if *is_synthetic {
                    InputOrigin::Game
                } else {
                    origin
                };
                self.note_input(
                    origin,
                    TraceKind::Key {
                        key: name.to_owned(),
                        action: action.to_owned(),
                    },
                );
            }
            WindowEvent::MouseInput { state, button, .. } => self.note_input(
                origin,
                TraceKind::Mouse {
                    button: keybinds::mouse_name(*button),
                    action: match state {
                        ElementState::Pressed => "down".to_owned(),
                        ElementState::Released => "up".to_owned(),
                    },
                },
            ),
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y, lines) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (f64::from(*x), f64::from(*y), true),
                    MouseScrollDelta::PixelDelta(p) => (p.x, p.y, false),
                };
                self.note_input(origin, TraceKind::Wheel { x, y, lines });
            }
            // GUI clicks land where the pointer is; in the world the pointer is
            // grabbed and turns the view through `device_event` instead.
            WindowEvent::CursorMoved { position, .. } if self.screen != Screen::Playing => {
                self.note_input(
                    origin,
                    TraceKind::Cursor {
                        x: position.x,
                        y: position.y,
                    },
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> TraceKind {
        TraceKind::Key {
            key: name.to_owned(),
            action: "down".to_owned(),
        }
    }

    #[test]
    fn the_ring_keeps_the_newest_and_counts_every_origin() {
        let mut trace = InputTrace::default();
        for i in 0..=(KEPT as u64 + 10) {
            trace.record(i, InputOrigin::User, key("key.keyboard.w"), false);
        }
        assert_eq!(trace.log.len(), KEPT);
        assert_eq!(trace.seq, KEPT as u64 + 11);
        assert_eq!(trace.log.front().unwrap().seq, 12);
        assert_eq!(trace.counts[InputOrigin::User.index()], KEPT as u64 + 11);
    }

    #[test]
    fn no_intrusion_without_an_agent() {
        let mut trace = InputTrace::default();
        assert!(!trace.record(1, InputOrigin::User, key("key.keyboard.w"), false));
        // The game's own input is not a person either.
        trace.record(2, InputOrigin::Agent, key("key"), false);
        assert!(!trace.record(3, InputOrigin::Game, key("key.keyboard.w"), false));
    }

    #[test]
    fn an_agent_still_watching_makes_a_later_person_an_intrusion() {
        let mut trace = InputTrace::default();
        trace.record(1, InputOrigin::Agent, key("key"), false);
        // The agent is waiting on a task or a wait: no five-second window.
        assert!(trace.record(2, InputOrigin::User, key("key.keyboard.a"), true));
    }

}

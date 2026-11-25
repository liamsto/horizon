//! What happened on the dedicated server, typed, for horizon-server's event stream (`GET /api/v1/events`,
//! docs/api/server.md): players joining, leaving and kicked, chat, and deaths. The log says the same in
//! vanilla's words; these carry the player's UUID and the parts apart, so a bot needn't parse lines.
//! `server-events SINCE` on the control endpoint answers the events after the SINCE-th.
use serde_json::Value;
use std::{collections::VecDeque, sync::Mutex};

/// The events kept for a reader that polls late.
const KEPT: usize = 1000;

struct Events {
    /// The number of the next event (the first is 1).
    next: u64,
    kept: VecDeque<Value>,
}

static EVENTS: Mutex<Events> = Mutex::new(Events {
    next: 1,
    kept: VecDeque::new(),
});

/// Records one event: its kind, the player's name and UUID, and what else it says.
pub(in crate::game) fn record(kind: &str, name: &str, uuid: u128, mut fields: Value) {
    fields["name"] = name.into();
    fields["uuid"] = horizon_protocol::crypto::uuid_string(uuid).into();
    push(kind, fields);
}

/// Records an event of the server itself, with no player (`saving` and `saved`, around a world save; an IP
/// ban), and what else it says.
pub(in crate::game) fn record_server(kind: &str, fields: Value) {
    push(kind, fields);
}

fn push(kind: &str, mut fields: Value) {
    let mut events = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    fields["seq"] = events.next.into();
    fields["type"] = kind.into();
    fields["time"] = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
        .into();
    events.next += 1;
    if events.kept.len() == KEPT {
        events.kept.pop_front();
    }
    events.kept.push_back(fields);
}

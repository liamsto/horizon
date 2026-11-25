//! Native debug renderers in a real game: when the renderer list refreshes, and what a frame's
//! world geometry holds as a renderer is switched and the world goes.
use super::{
    execution_role::ExecutionRole,
    mods,
    native_entity_refs::tests::{run_fresh_cases, with_client_game},
};
use glam::Mat4;
use horizon_client::{
    debug_entries::{DebugEntries, Status},
    world_geometry::WorldGeometry,
};
use horizon_protocol::packets::play::{self, Clientbound as C};
use mod_api::{
    ClientHost, DebugEntry, DebugGroup, DebugRenderer, DebugRendererDrawEvent, Hooks, ModEntry,
    RegisterDebugEntriesEvent, RegisterDebugRenderersEvent, WorldDepth, WorldLine,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const CAMERA: [f64; 3] = [100.0, 64.0, 100.0];
const SHOWN: &str = "marker:pipes";
const OTHER: &str = "marker:wires";

/// The ids `debug_renderer_draw` was asked about since the last frame.
static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// What the mod pushes at a refresh.
static PUSHED: Mutex<Vec<&str>> = Mutex::new(Vec::new());
/// The refreshes since the last look (`refreshes`).
static REFRESHES: AtomicUsize = AtomicUsize::new(0);

struct Marker;
#[mod_api::__subscriptions]
impl Hooks for Marker {
    fn register_debug_entries(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut RegisterDebugEntriesEvent<'_>,
    ) {
        for id in [SHOWN, OTHER] {
            event.entries.push(DebugEntry {
                id: id.into(),
                group: DebugGroup::Renderer,
            });
        }
    }
    fn register_debug_renderers(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut RegisterDebugRenderersEvent<'_>,
    ) {
        REFRESHES.fetch_add(1, Ordering::Relaxed);
        for id in PUSHED.lock().unwrap().iter() {
            event.renderers.push(DebugRenderer {
                id: (*id).to_owned(),
            });
        }
    }
    fn debug_renderer_draw(&self, _: &dyn ClientHost, event: &mut DebugRendererDrawEvent<'_>) {
        ASKED.lock().unwrap().push(event.id.to_owned());
        let marker = WorldLine::new(
            [100.25, 64.0, 99.0],
            [101.0, 64.0, 99.0],
            0xFF00FF00,
            2.0,
            WorldDepth::Tested,
        );
        event.draw.lines(&[marker]).unwrap();
    }
}

/// How many times the renderer list refreshed since the last call.
fn refreshes() -> usize {
    REFRESHES.swap(0, Ordering::Relaxed)
}

fn push(ids: &[&'static str]) {
    *PUSHED.lock().unwrap() = ids.to_vec();
}

// Mod declarations and the marker's state are process-global, not one Game's state.
fn fresh_process(name: &str) -> bool {
    run_fresh_cases(
        &format!("game::debug_renderer_tests::{name}"),
        "HORIZON_DEBUG_RENDERER_FIXTURE",
        &["run"],
    )
}

fn login() -> C {
    C::Login(play::Login {
        player_id: 7,
        hardcore: false,
        levels: vec!["minecraft:overworld".into()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 8,
        reduced_debug_info: false,
        show_death_screen: true,
        limited_crafting: false,
        spawn: play::SpawnInfo {
            dimension_type: 0,
            dimension: "minecraft:overworld".into(),
            seed: 1,
            game_type: 0,
            previous_game_type: None,
            debug: false,
            flat: false,
            last_death: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        online_mode: false,
        enforces_secure_chat: false,
    })
}

fn toggle_overlay(game: &mut super::Game<'_>) {
    use horizon_client::debug_keys::KeyAction;
    use winit::keyboard::KeyCode;
    game.debug_key(KeyCode::F3, KeyAction::Press);
    game.debug_key(KeyCode::F3, KeyAction::Release);
}

/// Prime a real world's view, change only the mod's renderer state, then invalidate through
/// the real input/packet path. The next view must collect the new list exactly once.
fn invalidation_refreshes(expected: &[&str], change: impl FnOnce(&mut super::Game<'_>)) {
    with_client_game(|game| {
        mod_platform::__private::install(vec![ModEntry::new("marker", Box::new(Marker))].into());
        mods::register_debug_entries(ExecutionRole::Client);
        game.debug_entries = DebugEntries::default();
        game.title = None;
        let (pipe, _server) = horizon_protocol::connection::Pipe::pair();
        game.connect_local(pipe, "Steve").unwrap();
        game.handle_packet(login()).unwrap();
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        game.debug_entries.set_status(OTHER, Status::AlwaysOn);
        let mut draw = WorldGeometry::default();
        push(&[SHOWN]);
        game.fill_world_geometry(
            false,
            CAMERA,
            Mat4::IDENTITY,
            0.0,
            &mut draw,
            &mut Vec::new(),
        );
        ASKED.lock().unwrap().clear();
        refreshes();
        push(&[OTHER]);
        change(game);
        game.fill_world_geometry(
            false,
            CAMERA,
            Mat4::IDENTITY,
            0.0,
            &mut draw,
            &mut Vec::new(),
        );
        assert_eq!(
            (refreshes(), std::mem::take(&mut *ASKED.lock().unwrap())),
            (1, expected.iter().map(|id| (*id).to_owned()).collect()),
            "next world view must replace the stale renderer list"
        );
        game.fill_world_geometry(
            false,
            CAMERA,
            Mat4::IDENTITY,
            0.0,
            &mut draw,
            &mut Vec::new(),
        );
        ASKED.lock().unwrap().clear();
        assert_eq!(refreshes(), 0, "no work on an unchanged frame");
    });
}

#[test]
#[ignore = "daily"]
fn debug_renderer_respawn_invalidates_unchanged_selection() {
    if fresh_process("debug_renderer_respawn_invalidates_unchanged_selection") {
        return;
    }
    invalidation_refreshes(&[OTHER], |game| {
        let C::Login(login) = login() else {
            unreachable!()
        };
        game.handle_packet(C::Respawn(play::Respawn {
            spawn: login.spawn,
            keep: 3,
        }))
        .unwrap();
    });
}

fn same_value_event(event: i8, expected: &[&str]) {
    invalidation_refreshes(expected, |game| {
        push(&[SHOWN]);
        game.handle_packet(C::EntityEvent(play::EntityEvent { id: 7, event }))
            .unwrap();
        let mut draw = WorldGeometry::default();
        game.fill_world_geometry(
            false,
            CAMERA,
            Mat4::IDENTITY,
            0.0,
            &mut draw,
            &mut Vec::new(),
        );
        ASKED.lock().unwrap().clear();
        refreshes();
        push(&[OTHER]);
        game.handle_packet(C::EntityEvent(play::EntityEvent { id: 7, event }))
            .unwrap();
    });
}

#[test]
#[ignore = "daily"]
fn debug_renderer_same_value_reduced_info_on_invalidates() {
    if fresh_process("debug_renderer_same_value_reduced_info_on_invalidates") {
        return;
    }
    same_value_event(22, &[]);
}

#[test]
#[ignore = "daily"]
fn debug_renderer_same_value_reduced_info_off_invalidates() {
    if fresh_process("debug_renderer_same_value_reduced_info_off_invalidates") {
        return;
    }
    same_value_event(23, &[OTHER]);
}

#[test]
#[ignore = "daily"]
fn debug_renderer_overlay_round_trip_invalidates() {
    if fresh_process("debug_renderer_overlay_round_trip_invalidates") {
        return;
    }
    invalidation_refreshes(&[OTHER], |game| {
        let before = game.debug;
        toggle_overlay(game);
        assert_ne!(game.debug, before);
        toggle_overlay(game);
        assert_eq!(game.debug, before);
    });
}

#[test]
#[ignore = "daily"]
fn debug_renderer_second_login_invalidates_first_world_view() {
    if fresh_process("debug_renderer_second_login_invalidates_first_world_view") {
        return;
    }
    invalidation_refreshes(&[OTHER], |game| {
        push(&[SHOWN]);
        game.handle_packet(login()).unwrap();
        assert_eq!(refreshes(), 1, "immediate handleLogin refresh");
        // A logging-in listener can update its state after the immediate refresh.
        push(&[OTHER]);
    });
}

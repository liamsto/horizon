//! Exercise the real local-player caller and the same listener through both ABIs.
use super::*;
use crate::game::native_entity_refs::tests::with_client_game;
use horizon_client::motion::PlayerMotion;
use mod_api::Hooks;
use winit::keyboard::KeyCode;

struct Native;
#[mod_api::__subscriptions]
impl Hooks for Native {
}

fn reset_player(game: &mut Game<'_>, food: i32) {
    let mut player = PlayerMotion::new(game.motion, Vec3::new(8.0, 64.0, 8.0)).unwrap();
    player.client_side = true;
    player.health.food.food = food;
    player.ground.on_ground = true;
    game.player = Some(player);
    game.keys.clear();
    game.using = None;
    let remote = game.remote.as_mut().unwrap();
    remote.health = (20.0, food, 5.0);
    remote.was_sprinting = false;
    remote.last_input = 0;
}

fn connect(game: &mut Game<'_>) -> horizon_protocol::connection::Pipe {
    let (client, server) = horizon_protocol::connection::Pipe::pair();
    game.connect_local(client, "movement-client").unwrap();
    let remote = game.remote.as_mut().unwrap();
    remote.logged_in = true;
    remote.loaded_sent = true;
    remote.entity_id = 42;
    remote.level_load.finish();
    game.screen = Screen::Playing;
    game.streaming
        .insert_remote(&mut game.world, horizon_client::chunk::Chunk::empty(0, 0));
    server
}

fn tick(game: &mut Game<'_>) {
    game.tick_local_player(Vec::new(), Vec3::ZERO, 7).unwrap();
}

#[test]
#[ignore = "daily"]
fn movement_input_without_listeners_keeps_vanilla_latch_double_tap_and_controller() {
    mod_platform::testing::with_mods(Vec::new().into(), || {
        with_client_game(|game| {
            let server = connect(game);
            let keys = game.mod_movement_keys(Keys {
                forward: true,
                analog: Some([0.2, 0.8]),
                ..Keys::default()
            });
            assert_eq!(keys.direction(), [0.2, 0.8]);
            assert!(!keys.sprint_suppressed);
            reset_player(game, 20);
            game.keys.insert(KeyCode::KeyW);
            game.keys.insert(KeyCode::ControlLeft);
            tick(game);
            assert!(game.player.as_ref().unwrap().sprinting);
            game.keys.remove(&KeyCode::ControlLeft);
            tick(game);
            assert!(game.player.as_ref().unwrap().sprinting, "vanilla latch");
            game.keys.clear();
            tick(game);
            assert!(!game.player.as_ref().unwrap().sprinting);
            reset_player(game, 20);
            game.keys.insert(KeyCode::KeyW);
            tick(game);
            assert!(!game.player.as_ref().unwrap().sprinting);
            game.keys.clear();
            tick(game);
            game.keys.insert(KeyCode::KeyW);
            tick(game);
            assert!(
                game.player.as_ref().unwrap().sprinting,
                "vanilla double tap"
            );
            server.shutdown();
        });
    });
}

#[test]
#[ignore = "daily"]
fn movement_input_disabled_listener_keeps_vanilla_keys() {
    mod_platform::testing::with_mods(
        vec![mod_api::ModEntry::new("movement", Box::new(Native))].into(),
        || {
            assert!(mod_api::__private::set_mod_enabled("movement", false));
            with_client_game(|game| {
                let keys = game.mod_movement_keys(Keys {
                    forward: true,
                    analog: Some([0.2, 0.8]),
                    ..Keys::default()
                });
                assert_eq!(keys.direction(), [0.2, 0.8]);
                assert!(!keys.sprint_suppressed);
                assert!(!keys.sprint);
                let keys = game.mod_movement_keys(Keys::default());
                assert_eq!(keys.direction(), [0.0, 0.0]);
                assert!(!keys.forward && !keys.sprint_suppressed);
                assert!(mod_api::__private::set_mod_enabled("movement", true));
                let keys = game.mod_movement_keys(Keys::default());
                assert_eq!(keys.direction(), [0.0, 1.0]);
                assert!(keys.forward && keys.sprint && !keys.sprint_suppressed);
                let keys = game.mod_movement_keys(Keys {
                    right: true,
                    ..Keys::default()
                });
                assert!(keys.forward && keys.right && keys.sprint_suppressed);
                assert!(mod_api::__private::set_mod_enabled("movement", false));
                let keys = game.mod_movement_keys(Keys::default());
                assert_eq!(keys.direction(), [0.0, 0.0]);
                assert!(!keys.forward && !keys.sprint_suppressed);
            });
        },
    );
}

#[path = "local_rotation_tests.rs"]
mod local_rotation_tests;

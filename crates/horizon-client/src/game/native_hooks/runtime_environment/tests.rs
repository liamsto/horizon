//! Bootstrap capture -> real registration -> real Game hosts, with the compiled wasm guest.
use crate::game::Options;
use horizon_mod_package::selection::LaunchArguments;

#[test]
fn normal_options_preserve_dedicated_flag_valued_operands() {
    let cwd = std::env::current_dir().unwrap();
    for option in ["world", "universe", "port", "serverId", "pidFile"] {
        let tail = vec![format!("--{option}"), "--game-tests".to_owned()];
        let launch = LaunchArguments::parse(
            std::iter::once("--dedicated".to_owned()).chain(tail.clone()),
            &cwd,
            &cwd,
            None,
        )
        .unwrap();
        let options = Options::from_launch(launch).unwrap().unwrap();
        assert_eq!(options.dedicated.as_ref(), Some(&tail));
        assert_eq!(
            crate::entry::dedicated_arguments::parse(options.dedicated.as_ref().unwrap())
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [(option, Some("--game-tests"))],
        );
    }
    let options = Options::parse(["--world", "--game-tests"].map(str::to_owned).into_iter())
        .unwrap()
        .unwrap();
    assert_eq!(options.world.as_deref(), Some("--game-tests"));
}

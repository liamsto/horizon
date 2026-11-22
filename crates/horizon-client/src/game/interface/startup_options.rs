//! The game-owned, side-effect-free startup validation boundary.
use super::*;

impl Options {
    /// The complete game-owned command-line boundary, before leases or native hooks.
    /// A bad command line is a [`launch_api::UsageError`] (exit code 2, no crash report).
    pub(crate) fn validate(launch: LaunchArguments) -> Result<Option<Self>> {
        let Some(mut o) =
            Self::from_launch(launch).map_err(|e| launch_api::UsageError(e.to_string()))?
        else {
            return Ok(None);
        };
        // Paths given on the command line stay relative to where the game was started.
        if o.game_dir.is_some() {
            o.resources = std::path::absolute(&o.resources)?;
            o.report = o.report.map(std::path::absolute).transpose()?;
        }
        Ok(Some(o))
    }
}

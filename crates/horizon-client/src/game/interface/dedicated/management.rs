//! The management protocol's list edits, which `horizon-server` forwards on the control endpoint as
//! `management <connection> {"method", "entries"}` (its management_methods.rs): vanilla 26.3's
//! `AllowlistService`, `BanlistService`, `IpBanlistService` and `OperatorService` on the running server's lists. Players are found as
//! `MinecraftPlayerListService.getUser` finds them, each change is logged as `JsonRpcLogger` logs it, and the
//! players a change shuts out are disconnected. The supervisor answers with the lists, from their files.
use crate::game::Game;

impl Game<'_> {

}

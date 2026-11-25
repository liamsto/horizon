//! Installed mod code loading above the engine (docs/layers.md, arismoko/horizon#2534).
//! The platform prepares the engine, loads mods here, then makes one engine start call.
#![deny(unsafe_code)]

#[allow(unsafe_code)]
mod native_libraries;
pub use native_libraries::{GAME_BUILD_ID, RUSTC_VERSION, main};

mod loading;
pub use loading::main_with;

/// Establish main-thread stack capacity before startup.
pub use horizon_guest_stack::prepare_main as prepare_main_stack;

/// A typed API-point listener supplied by generated native startup.
pub use mod_points::__private::SlotListener;

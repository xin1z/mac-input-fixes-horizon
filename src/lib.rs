//! The macOS wheel maths and configuration, with no game types.
//!
//! The native mixin in `src/mixins/wheel_head.rs` turns winit's
//! `MouseScrollDelta` into [`engine::Delta`], calls [`engine::process`], and
//! writes the result back as `MouseScrollDelta::LineDelta`. This crate only
//! holds the dependency-free maths and the config parser; the helper is
//! compiled inside the game's `window_handler` module by the launcher.

use mod_api::{self, Hooks};

pub mod engine;

/// The one `impl Hooks` at the crate root. This mod uses a native mixin and
/// no API hook, so every method keeps its default body.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {}

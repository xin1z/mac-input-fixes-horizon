// M0: prove the HEAD seam before any scroll maths are written.
//
// Compiled as a local function at the head of
// `Game::handle_window_event` (crates/horizon-client/src/game/window_handler.rs).
// It logs the wheel events a *person* produced and does nothing else, so a
// build here proves three things at once:
//   1. the injection target resolves at the pinned revision,
//   2. `&Game<'_>` from `&*self`, `&mut WindowEvent` from `&mut event` and
//      `InputOrigin` are all legal bindings for a helper,
//   3. the macOS-only guard compiles on every platform.
// Once this is green, the logger is replaced by the engine call (M1).
fn wheel_head(
    game: &crate::game::Game<'_>,
    event: &mut winit::event::WindowEvent,
    origin: crate::game::input_trace::InputOrigin,
) {
    if !cfg!(target_os = "macos") || origin != crate::game::input_trace::InputOrigin::User {
        return;
    }
    if let winit::event::WindowEvent::MouseWheel { delta, phase, .. } = event {
        // `scale_factor` is here because winit's `PixelDelta` is in physical
        // pixels: M2 needs it, and reading it proves the `Game` binding is
        // usable for more than standing in the signature.
        let scale = game.window.as_ref().map_or(1.0, |w| w.scale_factor());
        // The other three inputs M1/M3 need, probed (not used) so this build
        // proves they are reachable from a helper without accessor bridges.
        let hotbar = game.hotbar_takes_wheel();
        let attacking = game.left || game.right;
        let shift = game.modifiers.shift_key();
        eprintln!(
            "[macos-input-fixes] wheel delta={delta:?} phase={phase:?} scale_factor={scale} \
             hotbar={hotbar} attacking={attacking} shift={shift}"
        );
    }
}

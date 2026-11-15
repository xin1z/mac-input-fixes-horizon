// M1: fix MC-121772 (Shift + legacy wheel) and clamp a legacy wheel event to
// one notch. Compiled as a local function at the head of
// `Game::handle_window_event` (crates/horizon-client/src/game/window_handler.rs);
// the maths lives in the dependency-free `engine` crate so it can be tested
// without the game.
fn wheel_head(
    game: &crate::game::Game<'_>,
    event: &mut winit::event::WindowEvent,
    origin: crate::game::input_trace::InputOrigin,
) {
    use macos_input_fixes_engine as engine;

    if !cfg!(target_os = "macos") || origin != crate::game::input_trace::InputOrigin::User {
        return;
    }
    let winit::event::WindowEvent::MouseWheel { delta, phase, .. } = event else {
        return;
    };
    // M1 rewrites legacy wheel events only. MC-121772 is a legacy-wheel symptom
    // (macOS moves the delta onto the horizontal axis when Shift is held); a
    // precise trackpad / Magic Mouse gesture is left exactly as winit produced
    // it until M2 adds the phase and grouping state machine. Rewriting it here
    // would change its sensitivity and momentum without the M2 tests to back it.
    let winit::event::MouseScrollDelta::LineDelta(x, y) = *delta else {
        return;
    };
    let input = engine::WheelIn {
        delta: engine::Delta::Line(f64::from(x), f64::from(y)),
        phase: match *phase {
            winit::event::TouchPhase::Started => engine::Phase::Started,
            winit::event::TouchPhase::Moved => engine::Phase::Moved,
            winit::event::TouchPhase::Ended => engine::Phase::Ended,
            winit::event::TouchPhase::Cancelled => engine::Phase::Cancelled,
        },
        scale_factor: game.window.as_ref().map_or(1.0, |w| w.scale_factor()),
        zone: if game.hotbar_takes_wheel() {
            engine::Zone::Hotbar
        } else {
            engine::Zone::Gui
        },
        attacking: game.left || game.right,
        shift: game.modifiers.shift_key(),
        // winit 0.30 cannot expose macOS's momentum phase yet (plan §11.1).
        momentum: false,
    };
    // `None` would mean "swallow", which the plan writes as a zero line delta so
    // the event still reaches the screens (plan §4.2). M1 never returns `None`.
    let (x, y) = engine::process(input).unwrap_or((0.0, 0.0));
    *delta = winit::event::MouseScrollDelta::LineDelta(x as f32, y as f32);
}

// M1: fix MC-121772 (Shift + legacy wheel) and clamp a legacy wheel event to
// one notch. M2: group precise (trackpad / Magic Mouse) pixel deltas into
// whole notches and apply the display scale factor. Compiled as a local
// function at the head of `Game::handle_window_event`
// (crates/horizon-client/src/game/window_handler.rs); the maths lives in the
// dependency-free `engine` crate so it can be tested without the game.
fn wheel_head(
    game: &crate::game::Game<'_>,
    event: &mut winit::event::WindowEvent,
    origin: crate::game::input_trace::InputOrigin,
) {
    use macos_input_fixes_engine as engine;

    if !cfg!(target_os = "macos") || origin != crate::game::input_trace::InputOrigin::User {
        return;
    }
    // A partial notch from an interrupted gesture must not leak into the next
    // one (plan §6.5: reset when the window loses focus).
    if let winit::event::WindowEvent::Focused(false) = &*event {
        engine::reset();
        return;
    }
    let winit::event::WindowEvent::MouseWheel { delta, phase, .. } = event else {
        return;
    };
    // The config lives in the game directory (the launcher's `horizon` home,
    // not the repository) and is read once, on the first wheel event. A
    // missing or broken file leaves the defaults in place; restart to reload.
    engine::init_config(
        &horizon_client::logging::game_home().join("config/macos_input_fixes.toml"),
    );
    let engine_delta = match *delta {
        // Legacy wheel: already notches; shift folding and clamping apply.
        winit::event::MouseScrollDelta::LineDelta(x, y) => {
            engine::Delta::Line(f64::from(x), f64::from(y))
        }
        // Precise input: physical pixels; the engine converts them with the
        // window's scale factor and the configured points per notch.
        winit::event::MouseScrollDelta::PixelDelta(position) => {
            // `points_per_notch = 0` means "leave precise scrolling alone":
            // keep winit's PixelDelta so Horizon's own handling applies.
            if !engine::precise_scrolling_enabled() {
                return;
            }
            engine::Delta::Pixel(position.x, position.y)
        }
    };
    let input = engine::WheelIn {
        delta: engine_delta,
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
    // Emitting `LineDelta` bypasses Horizon's hard-coded `/30.0` pixel rule by
    // design; the engine already applied the scale factor (plan §5). `None`
    // would mean "swallow", written as a zero line delta so the event still
    // reaches the screens (plan §4.2). M1/M2 never return `None`.
    let (x, y) = engine::process(input).unwrap_or((0.0, 0.0));
    *delta = winit::event::MouseScrollDelta::LineDelta(x as f32, y as f32);
}

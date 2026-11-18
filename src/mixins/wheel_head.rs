// M2/M3 wheel handling, compiled as a HEAD helper inside
// `Game::handle_window_event_as` by the launcher's native mixin tool.
//
// No `use` lines: the helper sees the target module's imports, so `Game` and
// `input_trace` resolve here. It calls this mod's crate by its Rust name.
fn wheel_head(
    game: &crate::game::Game<'_>,
    event: &mut winit::event::WindowEvent,
    origin: crate::game::input_trace::InputOrigin,
) {
    if !cfg!(target_os = "macos") || origin != crate::game::input_trace::InputOrigin::User {
        return;
    }
    // A partial notch from an interrupted gesture must not leak into the next
    // one. The handler is also called for control-socket and replay events;
    // the origin gate above keeps those out.
    if let winit::event::WindowEvent::Focused(false) = &*event {
        macos_input_fixes::engine::reset();
        return;
    }
    let winit::event::WindowEvent::MouseWheel { delta, phase, .. } = event else {
        return;
    };
    // The config lives in the game directory, not the repository. It is read
    // once, on the first wheel event of a session.
    macos_input_fixes::engine::init_config(
        &horizon_client::logging::game_home().join("config/macos_input_fixes.toml"),
    );
    let engine_delta = match *delta {
        // Legacy wheel: already notches; shift folding and clamping apply.
        winit::event::MouseScrollDelta::LineDelta(x, y) => {
            macos_input_fixes::engine::Delta::Line(f64::from(x), f64::from(y))
        }
        // Precise input: physical pixels; the engine converts them with the
        // window's scale factor and the configured points per notch.
        winit::event::MouseScrollDelta::PixelDelta(position) => {
            if !macos_input_fixes::engine::precise_scrolling_enabled() {
                return;
            }
            macos_input_fixes::engine::Delta::Pixel(position.x, position.y)
        }
    };
    let input = macos_input_fixes::engine::WheelIn {
        delta: engine_delta,
        phase: match *phase {
            winit::event::TouchPhase::Started => macos_input_fixes::engine::Phase::Started,
            winit::event::TouchPhase::Moved => macos_input_fixes::engine::Phase::Moved,
            winit::event::TouchPhase::Ended => macos_input_fixes::engine::Phase::Ended,
            winit::event::TouchPhase::Cancelled => macos_input_fixes::engine::Phase::Cancelled,
        },
        scale_factor: game.window.as_ref().map_or(1.0, |w| w.scale_factor()),
        zone: if game.hotbar_takes_wheel() {
            macos_input_fixes::engine::Zone::Hotbar
        } else {
            macos_input_fixes::engine::Zone::Gui
        },
        attacking: game.left || game.right,
        shift: game.modifiers.shift_key(),
        // winit 0.30 folds macOS's momentum phase into TouchPhase; M4 is the
        // spike that separates them.
        momentum: false,
    };
    // Emitting `LineDelta` bypasses Horizon's hard-coded pixel scaling by
    // design; the engine already applied the scale factor. `None` means
    // "swallow", written as a zero line delta so the event still reaches the
    // screens.
    let (x, y) = macos_input_fixes::engine::process(input).unwrap_or((0.0, 0.0));
    *delta = winit::event::MouseScrollDelta::LineDelta(x as f32, y as f32);
}

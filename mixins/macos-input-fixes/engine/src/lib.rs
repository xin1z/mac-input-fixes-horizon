//! The macOS wheel maths, with no dependencies and no game types.
//!
//! The bundle's helper (`src/wheel_head.rs`) turns winit's `MouseScrollDelta`
//! into [`Delta`], calls [`process`], and writes the result back as
//! `MouseScrollDelta::LineDelta`. The unit on both sides is **one wheel
//! notch** at `mouseWheelSensitivity == 1.0`; Horizon applies the player's
//! sensitivity afterwards, so the engine must not.
//!
//! Ported from `processScroll` in MCMacOSInputFixes, lines 46–130 of
//! `src/main/native/com_hamarb123_macos_input_fixes_client_MacOSInputFixesClientMod.mm`
//! (BSD-3-Clause, © 2022 Hamish Arblaster); see `THIRD_PARTY_NOTICES.md` when
//! M5 adds it. The normative pseudocode is §6.1 of the port plan.
//!
//! M1 scope: `shift_wheel_fold` (MC-121772) and `clamp_legacy_wheel` only.
//! The trackpad state machine (`points_per_notch`, `Phase`, [`Gesture`]) is M2;
//! the remaining options are M3. They are declared here already so the helper
//! and the tests do not change shape between milestones.

use std::sync::{Mutex, MutexGuard};

/// Default trackpad sensitivity: GUI points per notch (M2 uses it).
pub const POINTS_PER_NOTCH: f64 = 20.0;

/// One wheel event, in the units winit gives the game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Delta {
    /// Wheel notches, as winit's `MouseScrollDelta::LineDelta`.
    Line(f64, f64),
    /// **Physical** pixels, as winit's `MouseScrollDelta::PixelDelta`.
    Pixel(f64, f64),
}

/// The gesture state winit reports for a precise (trackpad / Magic Mouse) event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// Where the wheel is going: an open screen, or the hotbar while playing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zone {
    Gui,
    Hotbar,
}

/// Everything one wheel event carries into the engine.
#[derive(Clone, Copy, Debug)]
pub struct WheelIn {
    pub delta: Delta,
    pub phase: Phase,
    /// The window's scale factor: winit's `PixelDelta` is in physical pixels.
    pub scale_factor: f64,
    pub zone: Zone,
    /// Attack or use is held (including toggle-attack / toggle-use).
    pub attacking: bool,
    pub shift: bool,
    /// The event came from post-release inertia (M4; pass `false` until then).
    pub momentum: bool,
}

/// The mod's options. Every field is a deliberate deviation from vanilla;
/// `shift_wheel_fold = false` is the one-line way back to modern macOS behaviour
/// for MC-121772 only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Fold macOS's Shift-moved horizontal delta back into the vertical axis.
    pub shift_wheel_fold: bool,
    /// A legacy wheel event is exactly ±1 notch (kills macOS acceleration jitter).
    pub clamp_legacy_wheel: bool,
    /// Trackpad points per notch; `0.0` leaves precise deltas alone (M2).
    pub points_per_notch: f64,
    /// Negate both axes everywhere (M3).
    pub reverse_scrolling: bool,
    /// Negate the vertical axis for the hotbar only (M3).
    pub reverse_hotbar: bool,
    /// Let post-release inertia change the hotbar (M3/M4).
    pub momentum_hotbar: bool,
    /// Swallow the wheel while attack or use is held (M3).
    pub lock_scroll_while_attacking: bool,
    /// Fold the horizontal axis into vertical for the hotbar (M3, mod parity).
    pub merge_horizontal_into_vertical: bool,
}

const DEFAULT_SETTINGS: Settings = Settings {
    shift_wheel_fold: true,
    clamp_legacy_wheel: true,
    points_per_notch: POINTS_PER_NOTCH,
    reverse_scrolling: false,
    reverse_hotbar: false,
    momentum_hotbar: false,
    lock_scroll_while_attacking: false,
    merge_horizontal_into_vertical: false,
};

impl Default for Settings {
    fn default() -> Self {
        DEFAULT_SETTINGS
    }
}

/// Per-gesture remainder for the trackpad grouping (M2 keeps it here).
#[derive(Clone, Copy, Debug, Default)]
pub struct Gesture {
    pub acc_x: f64,
    pub acc_y: f64,
}

static SETTINGS: Mutex<Settings> = Mutex::new(DEFAULT_SETTINGS);
static GESTURE: Mutex<Gesture> = Mutex::new(Gesture {
    acc_x: 0.0,
    acc_y: 0.0,
});

fn lock<T>(state: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panicking hook must not take the wheel down with it: keep the last
    // values instead of propagating the poison.
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `sgn` with the plan's definition: `0.0` stays `0.0` (Rust's `f64::signum`
/// returns `1.0` for zero, which would invent a notch). Non-finite input is
/// treated as no movement.
fn sgn(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        0.0
    } else {
        v.signum()
    }
}

/// Converts winit's units to notches.
fn raw_units(delta: Delta, scale_factor: f64, settings: &Settings) -> (f64, f64) {
    match delta {
        // A line delta is already notches.
        Delta::Line(x, y) => (x, y),
        Delta::Pixel(x, y) => {
            let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
                scale_factor
            } else {
                1.0
            };
            let per_notch = settings.points_per_notch;
            if per_notch.is_finite() && per_notch > 0.0 {
                (x / scale / per_notch, y / scale / per_notch)
            } else {
                // `0.0` disables custom trackpad handling: pass the pixels on.
                (x, y)
            }
        }
    }
}

/// Pure, testable core. `None` = swallow the wheel event.
///
/// M1 applies `shift_wheel_fold` and `clamp_legacy_wheel` to legacy
/// (`Delta::Line`) events and never swallows. The `Delta::Pixel` branch only
/// converts units here; the helper leaves precise gestures untouched until M2
/// adds the `Phase` state machine.
pub fn step(state: &mut Gesture, settings: &Settings, input: WheelIn) -> Option<(f64, f64)> {
    // M2 owns the gesture remainder; M1 must still accept the state so the
    // public shape does not change between milestones.
    let _ = state;

    let WheelIn {
        delta,
        scale_factor,
        shift,
        ..
    } = input;
    let legacy = matches!(delta, Delta::Line(..));
    let (mut x, mut y) = raw_units(delta, scale_factor, settings);

    // MC-121772: with Shift down, macOS moves the vertical delta onto the
    // horizontal axis for legacy events. Undo it (`.mm` lines 52–62).
    if legacy && settings.shift_wheel_fold && shift {
        y += x;
        x = 0.0;
    }

    // Legacy wheels report accelerated, fractional steps that vanilla would
    // read as multiple notches; one event is one notch (`.mm` lines 116–119).
    if legacy && settings.clamp_legacy_wheel {
        x = sgn(x);
        y = sgn(y);
    }

    // M1 has no option that swallows an event; M3's `lock_scroll_while_attacking`
    // and M4's momentum gate will return `None` from here.
    Some((x, y))
}

/// The helper's entry point: reads the process-wide settings and gesture state.
pub fn process(input: WheelIn) -> Option<(f64, f64)> {
    let settings = *lock(&SETTINGS);
    let mut gesture = lock(&GESTURE);
    step(&mut gesture, &settings, input)
}

/// Applies a new configuration and forgets the partial gesture (the plan's
/// §6.5: grouping state resets when the settings change).
pub fn set_settings(settings: Settings) {
    *lock(&SETTINGS) = settings;
    *lock(&GESTURE) = Gesture::default();
}

/// Forgets the partial gesture (tests; the helper may call it on focus loss).
pub fn reset() {
    *lock(&GESTURE) = Gesture::default();
}

#[cfg(test)]
mod tests;

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
//! M2 adds the `Started`/`Moved` grouping state machine for precise input
//! (`points_per_notch`, [`Phase`], [`Gesture`]). M3 adds `reverse_scrolling`,
//! `reverse_hotbar`, `lock_scroll_while_attacking`, `momentum_hotbar`,
//! `merge_horizontal_into_vertical` and the config file. Momentum *detection*
//! is M4; the gate is here.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Default trackpad sensitivity: GUI points per notch.
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

/// Whether precise (trackpad / Magic Mouse) deltas get custom handling.
/// `points_per_notch = 0` (or a nonsensical value) means "leave them alone":
/// the helper then passes winit's `PixelDelta` through untouched.
fn points_per_notch_enabled(points_per_notch: f64) -> bool {
    points_per_notch.is_finite() && points_per_notch > 0.0
}

/// Whether the helper should rewrite `MouseScrollDelta::PixelDelta` events.
pub fn precise_scrolling_enabled() -> bool {
    points_per_notch_enabled(lock(&SETTINGS).points_per_notch)
}

/// A whole number of notches, rounded towards zero (the `.mm`'s
/// `sgn(v) * (int)(|v| / sensitivity)` in notch units).
fn trunc_step(v: f64) -> f64 {
    if v.is_finite() { v.trunc() } else { 0.0 }
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
            if points_per_notch_enabled(settings.points_per_notch) {
                let per_notch = settings.points_per_notch;
                let to_notches = |v: f64| {
                    let notch = v / scale / per_notch;
                    if notch.is_finite() { notch } else { 0.0 }
                };
                (to_notches(x), to_notches(y))
            } else {
                // `0.0` disables custom trackpad handling: pass the pixels on.
                (x, y)
            }
        }
    }
}

/// Pure, testable core. `None` = swallow the wheel event.
///
/// Legacy (`Delta::Line`) events get `shift_wheel_fold` and
/// `clamp_legacy_wheel`; precise (`Delta::Pixel`) events get the original's
/// `Started`/`Moved` grouping (`.mm` lines 64–110). After that, in order:
/// the attack lock, the momentum gate, the optional hotbar merge, and the
/// reverse options.
pub fn step(state: &mut Gesture, settings: &Settings, input: WheelIn) -> Option<(f64, f64)> {
    let WheelIn {
        delta,
        phase,
        scale_factor,
        zone,
        attacking,
        shift,
        momentum,
    } = input;

    // The lock only applies to the hotbar: with a screen open the person is
    // not mining or using, and the original gates on the same condition.
    if settings.lock_scroll_while_attacking && attacking && zone == Zone::Hotbar {
        return None;
    }

    let legacy = matches!(delta, Delta::Line(..));
    let (mut x, mut y) = raw_units(delta, scale_factor, settings);

    // MC-121772: with Shift down, macOS moves the vertical delta onto the
    // horizontal axis for legacy events. Undo it (`.mm` lines 52–62).
    if legacy && settings.shift_wheel_fold && shift {
        y += x;
        x = 0.0;
    }

    if legacy {
        // Legacy wheels report accelerated, fractional steps that vanilla would
        // read as multiple notches; one event is one notch (`.mm` lines 116–119).
        if settings.clamp_legacy_wheel {
            x = sgn(x);
            y = sgn(y);
        }
    } else if points_per_notch_enabled(settings.points_per_notch) {
        // A precise gesture is a stream of small pixel deltas; group them into
        // whole notches so one flick is not a hundred scroll events (`.mm`
        // lines 64–110). `x`/`y` are already notches, so the threshold is 1.
        match phase {
            Phase::Started => {
                // The first event of a gesture always moves once, so a short
                // flick still changes a slot; keep the rest (never negative).
                let acc_x = sgn(x) * (x.abs() - 1.0).max(0.0);
                let acc_y = sgn(y) * (y.abs() - 1.0).max(0.0);
                *state = Gesture { acc_x, acc_y };
                x = sgn(x);
                y = sgn(y);
            }
            Phase::Moved | Phase::Ended | Phase::Cancelled => {
                state.acc_x += x;
                state.acc_y += y;
                x = 0.0;
                y = 0.0;
                if state.acc_x.abs() >= 1.0 {
                    x = trunc_step(state.acc_x);
                    state.acc_x = sgn(state.acc_x) * (state.acc_x.abs() % 1.0);
                    // A gesture is single-axis: the other axis' partial notch
                    // was probably not intended (`.mm` lines 93–108).
                    state.acc_y = 0.0;
                }
                if state.acc_y.abs() >= 1.0 {
                    y = trunc_step(state.acc_y);
                    state.acc_y = sgn(state.acc_y) * (state.acc_y.abs() % 1.0);
                    state.acc_x = 0.0;
                }
            }
        }
    }

    // Inertia must not turn the hotbar; GUI lists keep their vanilla inertia.
    // M4 supplies `momentum`; until then the helper passes `false`.
    if momentum && !settings.momentum_hotbar && zone == Zone::Hotbar {
        return None;
    }

    // The Java mod merges the axes for the hotbar so a Magic Mouse's horizontal
    // scroll picks a slot (MinecraftMixin.java:124–128). Horizon already falls
    // back to `-x` when `y == 0` (vanilla), so this is off by default.
    if settings.merge_horizontal_into_vertical && zone == Zone::Hotbar {
        y += x;
        x = 0.0;
    }

    if settings.reverse_scrolling {
        x = -x;
        y = -y;
    }
    if settings.reverse_hotbar && zone == Zone::Hotbar {
        y = -y;
    }

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

/// Set once the config file has been looked at; later wheel events skip the IO.
static CONFIG_LOADED: OnceLock<()> = OnceLock::new();

/// Reads the mod's config file once and applies it as the process-wide settings.
///
/// A missing file, an unreadable file and malformed lines all leave the
/// defaults in place: a broken config must not take the wheel down. Later calls
/// do nothing; restart the game to reload a changed file.
pub fn init_config(path: &Path) {
    CONFIG_LOADED.get_or_init(|| {
        if let Some(settings) = load_config(path) {
            set_settings(settings);
        }
    });
}

/// Reads and parses a config file; `None` when it cannot be read.
pub fn load_config(path: &Path) -> Option<Settings> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(parse_config(&text))
}

/// The config file's `key = value` subset, read over the defaults.
///
/// One setting per line; `#` starts a comment; blank lines are skipped; keys
/// are the [`Settings`] fields; unknown keys and malformed values are ignored.
/// There are no tables, arrays or strings, which is all this mod needs.
pub fn parse_config(text: &str) -> Settings {
    let mut settings = Settings::default();
    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = match line.find('#') {
            Some(index) => line[..index].trim(),
            None => line,
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        apply_setting(&mut settings, key.trim(), value.trim());
    }
    settings
}

fn apply_setting(settings: &mut Settings, key: &str, value: &str) {
    match key {
        "shift_wheel_fold" => set_bool(&mut settings.shift_wheel_fold, value),
        "clamp_legacy_wheel" => set_bool(&mut settings.clamp_legacy_wheel, value),
        "points_per_notch" => {
            if let Ok(notches) = value.parse::<f64>()
                && notches.is_finite()
            {
                settings.points_per_notch = notches;
            }
        }
        "reverse_scrolling" => set_bool(&mut settings.reverse_scrolling, value),
        "reverse_hotbar" => set_bool(&mut settings.reverse_hotbar, value),
        "momentum_hotbar" => set_bool(&mut settings.momentum_hotbar, value),
        "lock_scroll_while_attacking" => {
            set_bool(&mut settings.lock_scroll_while_attacking, value);
        }
        "merge_horizontal_into_vertical" => {
            set_bool(&mut settings.merge_horizontal_into_vertical, value);
        }
        _ => {}
    }
}

fn set_bool(target: &mut bool, value: &str) {
    match value.to_ascii_lowercase().as_str() {
        "true" => *target = true,
        "false" => *target = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests;

//! Table-driven tests for the M1 slice: `shift_wheel_fold` and
//! `clamp_legacy_wheel` on legacy deltas, plus the unit conversion the M2
//! helper will use. Cases are ported from the plan's §9.1 table and from
//! `processScroll` (MCMacOSInputFixes `.mm` lines 46–130), not from its code.

use super::*;

/// Settings variations, so the table stays readable.
#[derive(Clone, Copy, Debug)]
enum Tweak {
    Default,
    NoFold,
    NoClamp,
    FoldNoClamp,
    AllOff,
    ZeroPointsPerNotch,
}

fn settings(tweak: Tweak) -> Settings {
    let mut settings = Settings::default();
    match tweak {
        Tweak::Default => {}
        Tweak::NoFold => settings.shift_wheel_fold = false,
        Tweak::NoClamp | Tweak::FoldNoClamp => settings.clamp_legacy_wheel = false,
        Tweak::AllOff => {
            settings.shift_wheel_fold = false;
            settings.clamp_legacy_wheel = false;
        }
        Tweak::ZeroPointsPerNotch => settings.points_per_notch = 0.0,
    }
    settings
}

fn wheel(delta: Delta, shift: bool, scale_factor: f64) -> WheelIn {
    WheelIn {
        delta,
        phase: Phase::Moved,
        scale_factor,
        zone: Zone::Gui,
        attacking: false,
        shift,
        momentum: false,
    }
}

fn line(x: f64, y: f64) -> WheelIn {
    wheel(Delta::Line(x, y), false, 2.0)
}

fn line_with_shift(x: f64, y: f64) -> WheelIn {
    wheel(Delta::Line(x, y), true, 2.0)
}

fn pixel(x: f64, y: f64, scale_factor: f64) -> WheelIn {
    wheel(Delta::Pixel(x, y), false, scale_factor)
}

fn pixel_with_shift(x: f64, y: f64, scale_factor: f64) -> WheelIn {
    wheel(Delta::Pixel(x, y), true, scale_factor)
}

fn run(tweak: Tweak, input: WheelIn) -> (f64, f64) {
    let mut gesture = Gesture::default();
    step(&mut gesture, &settings(tweak), input).expect("M1 never swallows a wheel event")
}

struct Case {
    what: &'static str,
    input: WheelIn,
    tweak: Tweak,
    expected: (f64, f64),
}

#[test]
fn the_m1_table() {
    let cases = [
        // Legacy wheel: one event is exactly one notch on each axis.
        Case {
            what: "legacy wheel clamps each axis",
            input: line(3.0, -3.0),
            tweak: Tweak::Default,
            expected: (1.0, -1.0),
        },
        Case {
            what: "a zero legacy delta stays zero",
            input: line(0.0, 0.0),
            tweak: Tweak::Default,
            expected: (0.0, 0.0),
        },
        Case {
            what: "a fractional legacy step still moves one notch",
            input: line(0.4, -0.4),
            tweak: Tweak::Default,
            expected: (1.0, -1.0),
        },
        Case {
            what: "negative zero is not a notch",
            input: line(-0.0, 0.0),
            tweak: Tweak::Default,
            expected: (0.0, 0.0),
        },
        Case {
            what: "a NaN delta clamps to no movement",
            input: line(f64::NAN, 1.0),
            tweak: Tweak::Default,
            expected: (0.0, 1.0),
        },
        // MC-121772: Shift moves the delta onto the horizontal axis; fold it back.
        Case {
            what: "shift folds the horizontal axis into vertical",
            input: line_with_shift(1.0, 0.0),
            tweak: Tweak::Default,
            expected: (0.0, 1.0),
        },
        Case {
            what: "shift folds a negative horizontal step",
            input: line_with_shift(-1.0, 0.0),
            tweak: Tweak::Default,
            expected: (0.0, -1.0),
        },
        Case {
            what: "shift adds to an existing vertical step",
            input: line_with_shift(1.0, 2.0),
            tweak: Tweak::Default,
            expected: (0.0, 1.0),
        },
        Case {
            what: "a vertical wheel with shift is unchanged",
            input: line_with_shift(0.0, 3.0),
            tweak: Tweak::Default,
            expected: (0.0, 1.0),
        },
        Case {
            what: "fold off restores the reversed behaviour",
            input: line_with_shift(1.0, 0.0),
            tweak: Tweak::NoFold,
            expected: (1.0, 0.0),
        },
        // Clamp off is pass-through, per §6.3.
        Case {
            what: "clamp off passes the magnitude through",
            input: line(2.5, -2.5),
            tweak: Tweak::NoClamp,
            expected: (2.5, -2.5),
        },
        Case {
            what: "fold on with clamp off keeps the folded sum",
            input: line_with_shift(1.25, 0.0),
            tweak: Tweak::FoldNoClamp,
            expected: (0.0, 1.25),
        },
        Case {
            what: "every option off is the input verbatim",
            input: line_with_shift(1.25, -2.75),
            tweak: Tweak::AllOff,
            expected: (1.25, -2.75),
        },
        // Precise deltas: unit conversion only in M1 (no grouping until M2).
        Case {
            what: "pixels become notches on a 2x display",
            input: pixel(0.0, 40.0, 2.0),
            tweak: Tweak::Default,
            expected: (0.0, 1.0),
        },
        Case {
            what: "pixels become notches on a 1x display",
            input: pixel(0.0, 40.0, 1.0),
            tweak: Tweak::Default,
            expected: (0.0, 2.0),
        },
        Case {
            what: "the same points move the same amount on both displays",
            input: pixel(0.0, 80.0, 2.0),
            tweak: Tweak::Default,
            expected: (0.0, 2.0),
        },
        Case {
            what: "points_per_notch = 0 passes pixels through",
            input: pixel(40.0, 0.0, 2.0),
            tweak: Tweak::ZeroPointsPerNotch,
            expected: (40.0, 0.0),
        },
        Case {
            what: "shift never folds a precise delta",
            input: pixel_with_shift(20.0, 0.0, 2.0),
            tweak: Tweak::Default,
            expected: (0.5, 0.0),
        },
        Case {
            what: "a bad scale factor falls back to 1x",
            input: pixel(20.0, 0.0, 0.0),
            tweak: Tweak::Default,
            expected: (1.0, 0.0),
        },
    ];

    for case in cases {
        assert_eq!(
            run(case.tweak, case.input),
            case.expected,
            "case: {}",
            case.what
        );
    }
}

#[test]
fn defaults_are_the_documented_mod_defaults() {
    let settings = Settings::default();
    assert!(settings.shift_wheel_fold);
    assert!(settings.clamp_legacy_wheel);
    assert_eq!(settings.points_per_notch, 20.0);
    assert!(!settings.reverse_scrolling);
    assert!(!settings.reverse_hotbar);
    assert!(!settings.momentum_hotbar);
    assert!(!settings.lock_scroll_while_attacking);
    assert!(!settings.merge_horizontal_into_vertical);
}

/// The global tests share `SETTINGS`/`GESTURE`; keep them from overlapping.
static GLOBAL_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn process_reads_the_process_wide_settings() {
    let _guard = GLOBAL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    set_settings(Settings::default());
    reset();
    assert_eq!(process(line(3.0, -3.0)), Some((1.0, -1.0)));

    set_settings(Settings {
        shift_wheel_fold: false,
        ..Settings::default()
    });
    assert_eq!(process(line_with_shift(1.0, 0.0)), Some((1.0, 0.0)));

    // Leave the process-wide defaults for any later test or helper.
    set_settings(Settings::default());
    reset();
}

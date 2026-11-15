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
            input: pixel_with_shift(40.0, 0.0, 2.0),
            tweak: Tweak::Default,
            expected: (1.0, 0.0),
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

/// A precise event with an explicit phase; M2 groups these by gesture.
fn pixel_phase(x: f64, y: f64, scale_factor: f64, phase: Phase) -> WheelIn {
    WheelIn {
        phase,
        ..pixel(x, y, scale_factor)
    }
}

fn run_gesture(tweak: Tweak, events: &[WheelIn]) -> (Vec<(f64, f64)>, Gesture) {
    let settings = settings(tweak);
    let mut gesture = Gesture::default();
    let outputs = events
        .iter()
        .map(|&event| {
            step(&mut gesture, &settings, event).expect("M2 never swallows a wheel event")
        })
        .collect();
    (outputs, gesture)
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}

#[test]
fn a_started_trackpad_event_moves_once_and_seeds_the_remainder() {
    let (out, gesture) = run_gesture(
        Tweak::Default,
        &[pixel_phase(0.0, 40.0, 2.0, Phase::Started)],
    );
    assert_eq!(out, vec![(0.0, 1.0)]);
    assert_eq!((gesture.acc_x, gesture.acc_y), (0.0, 0.0));

    let (out, gesture) = run_gesture(
        Tweak::Default,
        &[pixel_phase(0.0, 70.0, 2.0, Phase::Started)],
    );
    assert_eq!(out, vec![(0.0, 1.0)]);
    close(gesture.acc_y, 0.75);
}

#[test]
fn moved_events_accumulate_until_a_whole_notch() {
    let events = [pixel_phase(0.0, 10.0, 2.0, Phase::Moved); 4];
    let (out, _) = run_gesture(Tweak::Default, &events);
    assert_eq!(out, vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 1.0)]);
}

#[test]
fn a_flick_with_a_started_event_moves_on_the_first_event() {
    let mut events = vec![pixel_phase(0.0, 20.0, 2.0, Phase::Started)];
    events.extend([pixel_phase(0.0, 10.0, 2.0, Phase::Moved); 4]);
    let (out, _) = run_gesture(Tweak::Default, &events);
    assert_eq!(
        out,
        vec![(0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 1.0)]
    );
}

#[test]
fn starting_a_new_gesture_forgets_the_partial_notch() {
    let events = [
        pixel_phase(32.0, 0.0, 2.0, Phase::Moved), // 0.8 notches kept
        pixel_phase(40.0, 0.0, 2.0, Phase::Started), // emits one, resets
        pixel_phase(32.0, 0.0, 2.0, Phase::Moved), // 0.8 again, below the threshold
    ];
    let (out, _) = run_gesture(Tweak::Default, &events);
    assert_eq!(out, vec![(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]);
}

#[test]
fn the_other_axis_remainder_is_discarded() {
    let events = [
        pixel_phase(32.0, 0.0, 2.0, Phase::Moved), // x accumulator 0.8
        pixel_phase(0.0, 44.0, 2.0, Phase::Moved), // y crosses: x remainder dropped
        pixel_phase(32.0, 0.0, 2.0, Phase::Moved), // no longer 0.8 + 0.8
    ];
    let (out, gesture) = run_gesture(Tweak::Default, &events);
    assert_eq!(out, vec![(0.0, 0.0), (0.0, 1.0), (0.0, 0.0)]);
    close(gesture.acc_x, 0.8);
}

#[test]
fn the_first_axis_to_cross_wins() {
    let (out, _) = run_gesture(
        Tweak::Default,
        &[pixel_phase(40.0, 40.0, 2.0, Phase::Moved)],
    );
    assert_eq!(out, vec![(1.0, 0.0)]);
}

#[test]
fn one_event_can_carry_several_notches() {
    let (out, gesture) = run_gesture(Tweak::Default, &[pixel_phase(0.0, 92.0, 2.0, Phase::Moved)]);
    assert_eq!(out, vec![(0.0, 2.0)]);
    close(gesture.acc_y, 0.3);
}

#[test]
fn the_remainder_keeps_its_sign() {
    let events = [
        pixel_phase(0.0, -56.0, 2.0, Phase::Moved), // -1.4 -> -1, remainder -0.4
        pixel_phase(0.0, 72.0, 2.0, Phase::Moved),  // -0.4 + 1.8 = 1.4 -> 1, not 2
    ];
    let (out, gesture) = run_gesture(Tweak::Default, &events);
    assert_eq!(out, vec![(0.0, -1.0), (0.0, 1.0)]);
    close(gesture.acc_y, 0.4);
}

#[test]
fn ended_and_cancelled_with_no_delta_move_nothing() {
    let events = [
        pixel_phase(0.0, 40.0, 2.0, Phase::Started),
        pixel_phase(0.0, 0.0, 2.0, Phase::Ended),
        pixel_phase(0.0, 0.0, 2.0, Phase::Cancelled),
    ];
    let (out, _) = run_gesture(Tweak::Default, &events);
    assert_eq!(out, vec![(0.0, 1.0), (0.0, 0.0), (0.0, 0.0)]);
}

#[test]
fn the_same_flick_moves_the_same_on_1x_and_2x() {
    // 80 physical pixels at 2x are the same 40 points as 40 pixels at 1x.
    let at_2x = run_gesture(
        Tweak::Default,
        &[
            pixel_phase(0.0, 80.0, 2.0, Phase::Started),
            pixel_phase(0.0, 0.0, 2.0, Phase::Moved),
        ],
    );
    let at_1x = run_gesture(
        Tweak::Default,
        &[
            pixel_phase(0.0, 40.0, 1.0, Phase::Started),
            pixel_phase(0.0, 0.0, 1.0, Phase::Moved),
        ],
    );
    assert_eq!(at_2x.0, at_1x.0);
    assert_eq!(at_2x.0, vec![(0.0, 1.0), (0.0, 1.0)]);
}

#[test]
fn points_per_notch_zero_keeps_precise_deltas_unhandled() {
    let (out, gesture) = run_gesture(
        Tweak::ZeroPointsPerNotch,
        &[pixel_phase(40.0, 0.0, 2.0, Phase::Moved)],
    );
    assert_eq!(out, vec![(40.0, 0.0)]);
    assert_eq!((gesture.acc_x, gesture.acc_y), (0.0, 0.0));
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

fn globals() -> MutexGuard<'static, ()> {
    GLOBAL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn process_reads_the_process_wide_settings() {
    let _guard = globals();

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

#[test]
fn precise_scrolling_is_enabled_unless_points_per_notch_is_zero() {
    let _guard = globals();
    set_settings(Settings::default());
    assert!(precise_scrolling_enabled());
    set_settings(Settings {
        points_per_notch: 0.0,
        ..Settings::default()
    });
    assert!(!precise_scrolling_enabled());
    set_settings(Settings::default());
}

#[test]
fn set_settings_forgets_the_partial_gesture() {
    let _guard = globals();
    set_settings(Settings::default());
    reset();
    // 0.8 notches: below the threshold, kept in the gesture.
    assert_eq!(process(pixel(0.0, 32.0, 2.0)), Some((0.0, 0.0)));
    // Changing the settings (even to the same values) must forget it.
    set_settings(Settings::default());
    // 0.9 notches alone must not become a whole notch.
    assert_eq!(process(pixel(0.0, 36.0, 2.0)), Some((0.0, 0.0)));
    set_settings(Settings::default());
    reset();
}

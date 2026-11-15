# macOS input fixes for Horizon

An unofficial native-mod port of [MCMacOSInputFixes] to [Horizon], the Rust
rewrite of Minecraft: Java Edition. It makes scrolling on macOS behave the way
players of the Fabric mod expect, instead of the way macOS's event translation
and vanilla's wheel handling leave it.

Not affiliated with or endorsed by the original author, Mojang, Microsoft or the
Horizon project. This is not a Fabric mod and shares no code with the Java
build: it is a source mixin bundle that Horizon's launcher compiles into the game.

## Status

Early development, on the way to a first release.

| Milestone | State |
|---|---|
| M0 — prove the `Game::handle_window_event` HEAD seam | done |
| M1 — Shift + legacy wheel, one notch per event | done |
| M2 — trackpad / Magic Mouse grouping, scale-aware notches | done |
| M3 — reverse scrolling, lock while attacking, config file | done |
| M4 — momentum gating (optional) | planned |
| M5 — release docs, registry listing | planned |

The engine has 28 unit test functions. The M1–M2 in-game checklist passed on
2026-11-16; the M3 options are unit-tested and still need an in-game pass.
Pinned to Horizon `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` (Nightly 2026-11-13).

## What it fixes

- **MC-121772 — Shift + scroll.** With Shift held, macOS moves a legacy wheel
  event's vertical delta onto the horizontal axis. Horizon, like vanilla, reads
  only the vertical axis, so lists did not scroll and the hotbar went the wrong
  way. The mod folds the axis back before Horizon sees the event.
- **Legacy wheel acceleration.** macOS can report several or fractional lines
  for one physical notch on a wheel mouse. The mod makes one legacy event
  exactly one notch, removing the "two slots per click" jitter.
- **Trackpad and Magic Mouse flicks.** Precise events are streams of tiny pixel
  deltas; Horizon turns every one into a wheel notch, so one flick could scroll
  a whole inventory and its speed varied with the flick. The mod groups a
  gesture into whole notches (`points_per_notch`, default 20 GUI points), emits
  one notch when the gesture starts, and keeps the remainder for the rest of it.
  It also divides by the window scale factor, so the same physical flick moves
  the same amount on a Retina and a 1× display.

## Already handled by Horizon (not touched here)

- Ctrl+click emulates a right click, off by default (MC-59810).
- Whole-stack drop reads Control, not Cmd (MC-22882).
- Ctrl+Tab / Ctrl+Esc are delivered by winit.
- The hotbar already falls back to the horizontal axis when vertical is zero
  (MC-122296).
- GUI inertial scrolling is not suppressed.

The mod also never rewrites synthetic input: `horizon-ctl`, replays and the
game's own pad menus arrive with an origin other than `User` and pass through
untouched.

## What it does not do yet

- **Momentum detection** (M4). After the fingers lift, inertia still scrolls GUI
  lists and can keep moving the hotbar: winit 0.30 does not expose macOS's
  momentum phase. The `momentum_hotbar` gate exists in the engine, but the
  helper always reports "no momentum" until M4.
- **Cmd for whole-stack drop.** The plan's `accept_cmd_for_drop_stack` option is
  deliberately not implemented: Horizon's quit shortcut is exactly Cmd+Q
  (`horizon_client::input::is_quit_shortcut_down`, with `quit_shortcuts` on by
  default), so dropping with Cmd would also be able to quit the game.
- Display-P3 colour space, which is not an input fix.

## Requirements

- macOS. The fixes are gated on `cfg!(target_os = "macos")`; on other platforms
  the bundle compiles and does nothing.
- Horizon built from the pinned commit above. Native mods are source mixins, so
  the launcher builds a game for the commit the mod pins.

## Install

From a checkout of this repository:

```sh
/Applications/Horizon.app/Contents/MacOS/horizon-launcher native-build \
  /path/to/this/repo/mixins/macos-input-fixes \
  --trust-native-code
```

`--trust-native-code` is the trust decision Horizon asks for: native mods run
with your user account's rights, so read the source first. The launcher builds a
game with the mod and selects it for the instance; start it with the launcher.

The bundle lives in `mixins/macos-input-fixes/`. The repository root is not
itself a bundle: Horizon's tooling requires a bundle directory for bundle
crates such as `engine/`.

## Uninstall

Remove the native build or the installed `.hnmod` in the launcher, then rebuild
the instance's installed mods:

```sh
/Applications/Horizon.app/Contents/MacOS/horizon-launcher native-build
```

The mod changes no game files on disk; the built game version is the only
artifact.

## Configuration

The mod reads one optional file from the game directory:

```
<game home>/config/macos-input-fixes.toml
# e.g. ~/Library/Application Support/horizon/config/macos-input-fixes.toml
```

Copy [`config/macos-input-fixes.toml.example`](config/macos-input-fixes.toml.example)
as a starting point. The file is read once, on the first wheel event of a
session; restart the game to reload it. A missing file, unknown keys, malformed
values and broken lines are ignored, so every unreadable setting keeps its
default and a bad config cannot break scrolling.

| Key | Default | Meaning |
|---|---|---|
| `shift_wheel_fold` | `true` | MC-121772: fold macOS's Shift-moved horizontal delta into vertical |
| `clamp_legacy_wheel` | `true` | one legacy wheel event is exactly one notch |
| `points_per_notch` | `20.0` | trackpad/Magic Mouse GUI points per notch; `0.0` leaves precise deltas to Horizon |
| `reverse_scrolling` | `false` | negate both axes everywhere |
| `reverse_hotbar` | `false` | negate the hotbar's vertical axis only |
| `lock_scroll_while_attacking` | `false` | swallow the wheel while attack or use is held (hotbar only) |
| `momentum_hotbar` | `false` | let post-release inertia move the hotbar; needs M4 detection, no-op today |
| `merge_horizontal_into_vertical` | `false` | the Java mod's hotbar merge; Horizon already falls back to `-x` |

## Deviations from vanilla

This is deliberately not a parity mod. Everything it changes is a deviation:

- Shift+wheel scrolls lists vertically instead of doing nothing.
- One legacy wheel event is exactly one notch, whatever macOS acceleration says.
- A trackpad flick is grouped into one or two notches instead of one notch per
  pixel event; the threshold is `points_per_notch` and is scale-aware.
- Reverse scrolling, reverse hotbar, the attack lock and the hotbar merge are
  extra options, off by default; turning them on is a deviation too.

## Development

```sh
# Engine unit tests (the crate has no dependencies):
cargo test --manifest-path mixins/macos-input-fixes/engine/Cargo.toml

# Compile the bundle into a staged Horizon and run the game crate's tests:
horizon-mixins check --workspace <horizon-source> --mods mixins \
  --out <fresh-stage> --trust-native-mods --package horizon-client
```

`NOTES.md` records the verified seam facts, decisions and evidence for each
milestone.

```
config/macos-input-fixes.toml.example   # for <game home>/config
mixins/macos-input-fixes/
├── horizon-mixin.toml     # the bundle manifest: one HEAD injection, one crate
├── src/wheel_head.rs      # the helper: winit event + config -> engine -> delta
└── engine/                # dependency-free maths, config parser + tests
```

## License

This repository's own code is MIT (see [LICENSE](LICENSE)).

Portions of the scroll-handling behaviour are derived from [MCMacOSInputFixes]
by Hamish Arblaster (hamarb123), BSD-3-Clause, Copyright (c) 2022 Hamish
Arblaster. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the full
notice.

[MCMacOSInputFixes]: https://github.com/hamarb123/MCMacOSInputFixes
[Horizon]: https://github.com/horizon-rust/horizon

# macOS input fixes for Horizon

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Release](https://img.shields.io/github/v/release/xin1z/mac-input-fixes-horizon?style=flat-square)](https://github.com/xin1z/mac-input-fixes-horizon/releases)
[![Platform: macOS](https://img.shields.io/badge/platform-macOS-lightgrey?style=flat-square)](#requirements)
[![Horizon: 26.3 Nightly 2026-11-18](https://img.shields.io/badge/Horizon-26.3%20Nightly%202026--11--18-blue?style=flat-square)](#requirements)
[![Signed with minisign](https://img.shields.io/badge/signed-minisign-brightgreen?style=flat-square)](https://github.com/xin1z/mac-input-fixes-horizon/blob/main/macos_input_fixes.pub)

An unofficial native-mod port of [MCMacOSInputFixes] to [Horizon], the Rust
rewrite of Minecraft: Java Edition. It ports the mod's macOS scrolling fixes, so
scrolling behaves the way players of the Fabric mod expect instead of the way
macOS's event translation and Horizon's wheel handling leave it.

Not affiliated with or endorsed by the original author, Mojang, Microsoft or the
Horizon project. This is not a Fabric mod and shares no code with the Java
build.

This repository is a **native `.hmod`**: a flat Rust project with a
`[native]` table in `horizon-mod.toml`, compiled by the launcher against one
exact Horizon build. It is *not* a sandboxed WebAssembly/content `.hmod`:
native code runs with the player's rights, outside Horizon's sandbox, which is
what a file-reading input fix needs. The old `.hnmod` format was retired by
Horizon's mod API merge; launchers from 0.2.70 on install this package.

## Status

Version 0.2.0 migrates the mod from the retired `.hnmod` format to native
`.hmod`. M0–M3 are implemented; M4 (momentum detection) is deferred as optional.
M5 covers the release docs and the native package.

| Milestone | State |
|---|---|
| M0 — prove the window-event wheel seam | done (ported to `handle_window_event_as`) |
| M1 — Shift + legacy wheel, one notch per event | done |
| M2 — trackpad / Magic Mouse grouping, scale-aware notches | done |
| M3 — reverse scrolling, lock while attacking, config file | done |
| M4 — momentum detection (optional) | deferred |
| M5 — release docs and native `.hmod` packaging | done; registry submission pending |

The engine has 28 unit test functions, all passing. The 0.1.0 `.hnmod` in-game
checklist passed on 2026-11-16; the 0.2.0 `.hmod` still needs an in-game pass on
launcher 0.2.70 or newer. Pinned to Horizon `dd67b76ff23e28c6d8e21859df600d31572f81a0`
(Nightly 2026-11-18), Horizon Mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

## What it fixes

- **MC-121772 — Shift + scroll.** With Shift held, macOS moves a legacy wheel
  event's vertical delta onto the horizontal axis. This is the old mouse-event
  path; trackpads are unaffected. Horizon's screens read the vertical axis (so
  they did not scroll), and the hotbar's fallback reads the horizontal axis with
  the opposite sign (so it was inverted). The mod folds the axis back before
  Horizon sees the event.
- **Legacy wheel acceleration.** macOS can report several or fractional lines
  for one physical notch on a wheel mouse. The mod makes one legacy event
  exactly one notch, removing the "two slots per click" jitter.
- **Trackpad and Magic Mouse flicks.** Precise events are streams of tiny pixel
  deltas; Horizon reads each event's pixels as wheel movement (divided by 30),
  so one flick could scroll far more rows than intended, and how far depended on
  the flick. The mod groups a gesture into whole notches (`points_per_notch`,
  default 20 GUI points), emits one notch when the gesture starts, and keeps the
  remainder for the rest of it. It also divides by the window scale factor, so
  the same physical flick moves the same amount on a Retina and a 1× display.

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
  lists and can keep moving the hotbar: winit 0.30 folds macOS's momentum phase
  into `TouchPhase`, so it cannot be told apart reliably. The `momentum_hotbar`
  gate exists in the engine, but the helper always reports "no momentum" until
  M4.
- **Cmd for whole-stack drop.** An `accept_cmd_for_drop_stack` option is
  deliberately not implemented: Cmd+Q quits the game by default, so dropping
  with Cmd would also be able to quit the game.
- **An in-game settings page.** There is no documented package-level way to
  declare option rows, so this release reads the config file only.
- Display-P3 colour space, which is not an input fix.

## Requirements

- macOS. The helper is gated on `cfg!(target_os = "macos")`; on other platforms
  it does nothing.
- Horizon `dd67b76ff23e28c6d8e21859df600d31572f81a0` (Nightly 2026-11-18,
  Minecraft 26.3), with its matching SDK. The manifest pins the exact Horizon
  and Horizon Mixins revisions.
- Horizon Launcher **0.2.70 or newer**. Launchers before 0.2.70 cannot load
  `.hmod` native packages; they only knew the retired `.hnmod` format.
- Native code must be allowed for the install (`--trust-native-code`). The
  first install shows a trust notice; native mods run with your user account's
  rights. Read the source before allowing it.

A newer Nightly needs a re-pin (`cargo horizon-mod update`) and may need changes
to the mixin target; a mod is made for one exact build, and the launcher refuses
to build it for another if anything it uses changed.

## Install

### From a release

Download `macos_input_fixes-0.2.0.hmod` (and its `.minisig` if you want to
verify it) from the [releases page]. In the launcher, open **Mods**, install the
`.hmod` from file, add it to the selected instance, and choose **Build**. On a
fresh install, allow native code when the trust notice appears.

The release file is source-only: the launcher fetches the pinned SDK and
compiles it on your machine. It does not contain a prebuilt game.

### From a checkout

```sh
# Fetch the pinned SDK and build the .hmod in target/horizon-mod/:
cargo horizon-mod build

# Or build it into the selected instance directly:
/Applications/Horizon.app/Contents/MacOS/horizon-launcher native-build . \
  --instance "My Instance" --trust-native-code
```

`native-build` compiles the crate against the pinned Horizon build, installs the
result as a native build, and selects it for the instance. Start the instance
from the launcher afterwards. `--trust-native-code` records the trust decision;
without it the launcher refuses to build native code.

### From the registry

The registry listing is being resubmitted as a native `.hmod`; the old `.hnmod`
submission was closed when the format was retired. Once listed, the launcher
installs it from the Mods page and checks the file's SHA-256 and signature.

## Configuration

The mod reads one optional file from the game directory:

```
<game home>/config/macos_input_fixes.toml
# e.g. ~/Library/Application Support/horizon/config/macos_input_fixes.toml
```

Copy [`config/macos_input_fixes.toml.example`](https://github.com/xin1z/mac-input-fixes-horizon/blob/main/config/macos_input_fixes.toml.example)
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

## How it works

The flat native project is:

```
horizon-mod.toml                 # identity, [native] pins, one [[native.mixin]] row
Cargo.toml                       # one crate, depends on the pinned SDK's mod API
src/lib.rs                       # the required Hooks impl (no API hooks used)
src/mixins/wheel_head.rs         # HEAD helper, compiled into the game by the launcher
src/engine/mod.rs                # dependency-free maths and config parser
src/engine/tests.rs              # 28 unit tests
icon.png                         # launcher / registry icon
```

The helper is a `HEAD` mixin on
`horizon_client::game::window_handler::Game::handle_window_event_as`. It checks
that the event came from the window's user (`InputOrigin::User`), resets the
partial notch when the window loses focus, reads the config once, maps winit's
line/pixel delta and gesture phase into the engine, and writes the result back
as `MouseScrollDelta::LineDelta`. Writing `LineDelta` bypasses Horizon's
hard-coded pixel scaling by design: the engine has already applied the scale
factor and the points-per-notch threshold.

The engine has no game types and no dependencies; the same maths can be tested
without compiling Horizon. A zero result means "swallow the wheel"; the helper
writes `LineDelta(0, 0)` so the event still reaches the screens.

## Deviations from vanilla

This is deliberately not a parity mod. Everything it changes is a deviation:

- Shift+wheel scrolls lists vertically instead of doing nothing.
- One legacy wheel event is exactly one notch, whatever macOS acceleration says.
- A trackpad flick is grouped into (typically) one or two notches instead of
  every tiny pixel event scrolling immediately; the threshold is
  `points_per_notch` and is scale-aware.
- Reverse scrolling, reverse hotbar, the attack lock and the hotbar merge are
  extra options, off by default; turning them on is a deviation too.

## Uninstall

Remove the mod from the launcher's Mods page and rebuild the instance's native
build (or delete the native build entry). You can also run:

```sh
/Applications/Horizon.app/Contents/MacOS/horizon-launcher native-build
```

The mod changes no world, save or options file. A native build and the
launcher's mod metadata are the only artifacts; uninstalling the mod leaves the
worlds and config untouched.

## Verifying a release

Release files are signed with [minisign](https://jedisct1.github.io/minisign/).
The public key is [macos_input_fixes.pub](https://github.com/xin1z/mac-input-fixes-horizon/blob/main/macos_input_fixes.pub) (fingerprint
`AF7C7026617FEF25`). Download the `.hmod` and its `.minisig` from the release,
then:

```sh
minisign -V -p macos_input_fixes.pub -m macos_input_fixes-0.2.0.hmod
# Signature and comment signature verified
```

The registry entry carries the same key and signature; for registry installs
the launcher checks both before installing. The secret key is never published.

## Development

```sh
# Engine and crate tests; the SDK is fetched into .horizon/sdk/ by any
# `cargo horizon-mod` command in a fresh clone.
cargo test

# Build the source-only package in target/horizon-mod/.
cargo horizon-mod build

# What the registry checks of the package (manifest, layout, hashes).
cargo horizon-mod check target/horizon-mod/macos_input_fixes.hmod
```

The mixin helper is not a module of `src/lib.rs`: the launcher compiles it inside
the game's `window_handler` module, where it can see `Game`'s private fields. The
mod's own crate only provides the engine and the required `Hooks` impl, so it
can be unit-tested without a game build.

## License

This repository's own code is MIT (see [LICENSE](LICENSE)).

Portions of the scroll-handling behaviour are derived from [MCMacOSInputFixes]
by Hamish Arblaster (hamarb123), BSD-3-Clause, Copyright (c) 2022 Hamish
Arblaster. See [NOTICE.md](NOTICE.md) for the full notice.

[MCMacOSInputFixes]: https://github.com/hamarb123/MCMacOSInputFixes
[Horizon]: https://github.com/horizon-rust/horizon
[releases page]: https://github.com/xin1z/mac-input-fixes-horizon/releases

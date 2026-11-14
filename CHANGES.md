# Changes

## 1.0.0 — November 2026

A complete rewrite in Rust with [GPUI](https://gpui.rs). The Java/JavaFX application and
its Maven modules are gone; the new application is a native binary with a new,
Razer Synapse-inspired dark interface.

### New

- Rust workspace: `rusted-snake-core`, `rusted-snake-backend`, `rusted-snake-ui`, and the
  `rusted-snake` binary.
- GPUI interface with a device rail, live OpenRazer status, cards, chips, sliders and a
  neon-green theme (`#44d62c`) on layered charcoal surfaces.
- Devices page with model, firmware, serial, battery, DPI, polling rate and live device
  discovery.
- Buttons page with onboard profile slots, Naga side plate selection, a mouse diagram with
  clickable markers, and a button editor drawer.
- Onboard profile reading, writing and verification ported to Rust over raw HID, including
  read-only probing for unknown Razer mice.
- Automatic backup of every onboard profile before the first write, plus backup/restore
  from Settings. Backups remain compatible with Snake's JSON format.
- Performance page with onboard DPI stages and live DPI/polling-rate controls.
- Lighting page with overall and per-zone brightness, hardware effects, effect colours,
  speed/direction controls and a custom HSV colour picker.
- Software spectrum, breath and audio-reactive effects for static-only devices. Audio
  capture uses PipeWire/PulseAudio through `parec`; the default source follows what is
  playing.
- Macro sets and macros that press a key, type text or run a command, with Once,
  While-held and Toggle repeat modes. The macro engine grabs Razer input devices with
  evdev and injects through a uinput virtual keyboard, then releases everything on exit.
- Settings page with experimental onboarding writes, startup-with-the-computer, permission
  instructions, OpenRazer diagnostics and the about panel.
- `bin/rusted-snake` launcher that builds on first run, fast-forwards from git when clean,
  and starts the release binary.
- Ship udev rules for raw HID and uinput access, a desktop entry, and app icons.
- `--page` argument for opening a specific page.

### Removed

- Java, JavaFX, Maven, the old theme modules, the old images and the old launcher.
- The tray icon and the in-app updater. The launcher handles updates; live effects and
  macros run while the window is open.
- Per-app macro lists, macro recording and the classic layout designer from older Snake
  versions.

Older history (up to 2021) is in the git log and at
https://github.com/bithatch/snake. The intermediate Linux fork is at
https://github.com/ScoobyDouche/snake.

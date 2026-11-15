# Copyright, attribution and notices

**Rusted Snake** — a Razer Synapse-style device configurator for Linux.

Copyright © 2026 **Henry Chea**.

Rusted Snake is free software: you can redistribute it and/or modify it under the terms of
the GNU General Public License as published by the Free Software Foundation, version 3 of
the License. It is distributed in the hope that it will be useful, but **without any
warranty**; without even the implied warranty of merchantability or fitness for a particular
purpose. See [LICENSE](LICENSE) for the full license text.

## Upstream Snake

Rusted Snake is a Rust/GPUI rewrite of the Linux fork of **Snake**:

- **Snake** — Copyright © 2020–2021 Brett Smith and contributors.
  <https://github.com/bithatch/snake>
- **ScoobyDouche/snake** — Linux fork, Copyright © 2026 ScoobyDouche and contributors.
  <https://github.com/ScoobyDouche/snake>

The original Java/JavaFX sources are not included in this repository. The Rust rewrite
implements the same device behavior, onboard-profile protocol and backup format, and keeps
the upstream GPL-3.0 licensing and attribution. The original mouse-photo screenshots are not
included; the artwork in this repository is new.

## Vendored components

| Component | Copyright | License | Location |
| --- | --- | --- | --- |
| GPUI 0.2.2 | Copyright © Zed Industries and contributors | Apache-2.0 | `vendor/gpui` |
| GPUI Linux event-loop patch | Copyright © 2026 Henry Chea | Apache-2.0 (as part of GPUI) | `vendor/gpui/README.rusted-snake.md` |
| xattr 0.2.3 | Copyright © Steven Allen | MIT OR Apache-2.0 | `vendor/xattr` |
| xattr libc patch | Copyright © 2026 Henry Chea | MIT OR Apache-2.0 (as part of xattr) | `vendor/xattr/README.md` |
| Barlow fonts | Copyright © 2017 The Barlow Project Authors | SIL Open Font License 1.1 | `assets/fonts` |

License texts are kept next to each component. The vendored sources contain their original
copyright and license headers; local patches are documented in the component README files.

## Rust dependencies

Direct dependencies include GPUI, `ksni`, `dbus`, `serde`, `crossbeam-channel`, `libc`,
`chrono`, `image` and `log`. Their copyright notices and licenses remain with their authors.
See `Cargo.lock` for the complete dependency graph and run `cargo metadata` or use a license
scanner for the full texts. `ksni` is Unlicense; `image`, `serde`, `crossbeam-channel`,
`libc` and `log` are typically MIT/Apache-2.0 dual-licensed.

## External programs

Rusted Snake talks to external programs; it does not vendor or link their code:

- **OpenRazer** (`openrazer-daemon`, drivers and udev rules) — GPL-2.0, by the OpenRazer
  project. Rusted Snake uses its D-Bus API.
- **OpenRGB** — GPL-2.0, by the OpenRGB project. The OpenRGB lighting tab invokes the
  `openrgb` command line; no OpenRGB client library is linked.
- **PipeWire/PulseAudio** tools (`parec`, `pactl`) — used for audio-reactive lighting.

Their copyrights, licenses and trademarks remain with their respective authors.

## Artwork and names

- The snake logo, app icon, mouse illustration, banner, palette card, icons and UI theme are
  original work, Copyright © 2026 Henry Chea, licensed under GPL-3.0-only with the rest
  of this project.
- The Barlow font family is Copyright © 2017 The Barlow Project Authors and is included under
  the SIL Open Font License 1.1. See `assets/fonts/OFL.txt`.

## Trademarks

Razer, Razer Synapse, Razer Chroma, Naga and related marks are trademarks or registered
trademarks of Razer Inc. OpenRazer and OpenRGB are independent projects. Rusted Snake is an
independent community project and is **not affiliated with, endorsed by, or sponsored by
Razer Inc.**

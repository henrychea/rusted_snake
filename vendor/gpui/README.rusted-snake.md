# GPUI 0.2.2 (patched)

Vendored to keep the event loop alive when the last visible window is hidden to the
system tray. Upstream Linux clients call `signal.stop()` as soon as the window list
becomes empty, which would terminate Rusted Snake while it is in the tray and make it
impossible to reopen from the StatusNotifierItem.

The patch removes that automatic stop in:

- `src/platform/linux/wayland/client.rs`
- `src/platform/linux/x11/client.rs`

`Platform::quit()` still stops the loop, so explicit quit (tray menu or Ctrl+Q) works.

The patch is Copyright © 2026 Henry Chea and is distributed under the GPUI crate's Apache-2.0 license.

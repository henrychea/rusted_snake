# xattr 0.2.3 (patched)

Vendored only to fix a build failure on current Rust/libc:

`src/sys/mod.rs` used `libc::ENOATTR`, which recent libc releases no longer define on
Linux. Linux defines `ENOATTR` as `ENODATA`, so the patch uses `ENODATA` on Linux/Android
and keeps `ENOATTR` elsewhere.

This crate is a transitive dependency of GPUI's HTTP client. Remove the patch when GPUI
updates `zed-async-tar`/`xattr` to a release that builds against current libc.

The local patch is Copyright © 2026 Henry Chea and is distributed under the same MIT OR Apache-2.0 terms as xattr.

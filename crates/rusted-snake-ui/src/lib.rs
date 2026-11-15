// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

//! GPUI interface for Rusted Snake.

pub mod app;
pub mod assets;
pub mod pages;
pub mod theme;
pub mod tray;
pub mod widgets;

pub use app::{Page, Root, open, open_page};

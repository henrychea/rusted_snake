// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use gpui::Application;
use rusted_snake_ui::assets::{Assets, fonts};

fn main() {
    if std::env::args().any(|argument| argument == "--version" || argument == "-V") {
        println!("rusted-snake {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args().any(|argument| argument == "--help" || argument == "-h") {
        println!("Rusted Snake {}", env!("CARGO_PKG_VERSION"));
        println!(
            "A Razer Synapse-style device configurator for OpenRazer, written in Rust with GPUI."
        );
        println!();
        println!(
            "Usage: rusted-snake [--page devices|buttons|performance|lighting|macros|settings]"
        );
        println!();
        println!("Settings and macros live in ~/.config/rusted-snake.");
        return;
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let mut page = rusted_snake_ui::Page::Devices;
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(index) = arguments.iter().position(|argument| argument == "--page")
        && let Some(value) = arguments.get(index + 1)
        && let Some(selected) = rusted_snake_ui::Page::parse(value)
    {
        page = selected;
    }
    Application::new().with_assets(Assets).run(move |cx| {
        if let Err(error) = cx.text_system().add_fonts(fonts()) {
            log::error!("could not load Barlow: {error}");
        }
        rusted_snake_ui::open_page(cx, page);
    });
}

// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

//! StatusNotifierItem tray icon. GPUI has no tray API, so this uses the
//! freedesktop/KDE StatusNotifierItem protocol through ksni.

use crossbeam_channel::{Receiver, Sender, unbounded};
use ksni::Tray;
use ksni::blocking::Handle;
use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};

use crate::app::Page;

#[derive(Clone, Debug)]
pub enum TrayCommand {
    Show,
    Page(Page),
    Quit,
}

pub struct TrayRuntime {
    pub commands: Receiver<TrayCommand>,
    handle: Handle<AppTray>,
}

impl Drop for TrayRuntime {
    fn drop(&mut self) {
        self.handle.shutdown().wait();
    }
}

struct AppTray {
    sender: Sender<TrayCommand>,
    icon: Vec<ksni::Icon>,
}

impl AppTray {
    fn send(&self, command: TrayCommand) {
        let _ = self.sender.send(command);
    }
}

impl Tray for AppTray {
    const MENU_ON_ACTIVATE: bool = false;

    fn id(&self) -> String {
        "rusted-snake".into()
    }

    fn title(&self) -> String {
        "Rusted Snake".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icon.clone()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayCommand::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = vec![
            StandardItem {
                label: "Show Rusted Snake".into(),
                activate: Box::new(|this: &mut Self| this.send(TrayCommand::Show)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];
        for page in [
            Page::Devices,
            Page::Buttons,
            Page::Performance,
            Page::Lighting,
            Page::Macros,
            Page::Settings,
        ] {
            let page_choice = page;
            items.push(
                StandardItem {
                    label: page.label().into(),
                    activate: Box::new(move |this: &mut Self| {
                        this.send(TrayCommand::Page(page_choice))
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| this.send(TrayCommand::Quit)),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

/// Start the tray icon. Returns `None` when the desktop has no StatusNotifier
/// host; the window then closes normally instead of minimising to a tray that
/// does not exist.
pub fn start() -> Option<TrayRuntime> {
    let (sender, commands) = unbounded();
    let tray = AppTray {
        sender,
        icon: load_icon(),
    };
    match tray.spawn() {
        Ok(handle) => Some(TrayRuntime { commands, handle }),
        Err(error) => {
            log::warn!("status notifier tray unavailable: {error}");
            None
        }
    }
}

fn load_icon() -> Vec<ksni::Icon> {
    let bytes = include_bytes!("../../../assets/icon/rusted-snake.png");
    let Ok(image) = image::load_from_memory(bytes) else {
        return Vec::new();
    };
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in image.pixels() {
        // StatusNotifierItem expects ARGB32 in network byte order.
        data.extend_from_slice(&[pixel[3], pixel[0], pixel[1], pixel[2]]);
    }
    vec![ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }]
}

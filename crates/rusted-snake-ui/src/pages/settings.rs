use gpui::prelude::*;
use gpui::{AnyElement, Context, FontWeight, Window, div, px};

use crate::app::{Page, Root};
use crate::theme;
use crate::widgets;
use crate::widgets::OnPress;
use rusted_snake_backend::EngineState;

impl Root {
    pub(crate) fn settings_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut page = div()
            .flex()
            .flex_col()
            .gap_5()
            .p(px(28.))
            .pb(px(44.))
            .max_w(px(1100.))
            .w_full()
            .child(self.page_header(
                "Settings",
                "Device access, startup, backups and diagnostics.",
                "icons/settings.svg",
            ));

        // OpenRazer connection
        let mut devices = div().flex().flex_col().gap_2();
        if self.devices.is_empty() {
            devices = devices.child(widgets::hint("No devices are connected right now."));
        }
        for device in self.devices.clone() {
            devices = devices.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py(px(7.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::icon(device.kind.icon(), 14.0, theme::muted()))
                            .child(widgets::value(device.display_name.clone())),
                    )
                    .child(widgets::mono_value(format!(
                        "{:04x}:{:04x}",
                        device.vid, device.pid
                    ))),
            );
        }
        page = page.child(
            widgets::card_padded()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(widgets::heading("OpenRazer"))
                                .child(widgets::hint(
                                    "Rusted Snake talks to devices through the OpenRazer daemon. Device changes are applied live.",
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(widgets::pill(
                                    if self.backend_info.connected { "CONNECTED" } else { "OFFLINE" },
                                    if self.backend_info.connected { theme::accent() } else { theme::danger() },
                                ))
                                .child(
                                    widgets::icon_button("icons/refresh.svg").on_press(cx.listener(
                                        |this, _, _, cx| {
                                            this.backend.send(rusted_snake_backend::Command::RefreshDevices);
                                            this.show_toast("Refreshing devices...".into(), false, cx);
                                        },
                                    )),
                                ),
                        ),
                )
                .child(widgets::info_row(
                    "Daemon version",
                    if self.backend_info.version.is_empty() {
                        "unknown".into()
                    } else {
                        self.backend_info.version.clone()
                    },
                ))
                .child(widgets::divider())
                .child(devices)
                .children(self.backend_info.error.clone().map(|error| {
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(theme::with_alpha(0x000000, 0.25))
                        .child(widgets::icon("icons/alert.svg", 14.0, theme::warning()))
                        .child(widgets::hint(error))
                })),
        );

        page = page.child(
            widgets::card_padded()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(widgets::heading("Device access"))
                        .child(widgets::hint(
                            "Onboard profiles use the mouse's raw HID node, and macros need the input devices plus /dev/uinput. These rules grant access to the signed-in user.",
                        )),
                )
                .child(widgets::section_label("ONBOARD PROFILES"))
                .child(widgets::mono_value("sudo cp linux/70-rusted-snake-hidraw.rules /etc/udev/rules.d/"))
                .child(widgets::mono_value("sudo udevadm control --reload-rules && sudo udevadm trigger"))
                .child(widgets::divider())
                .child(widgets::section_label("MACROS"))
                .child(widgets::mono_value("sudo cp linux/99-rusted-snake-uinput.rules /etc/udev/rules.d/"))
                .child(widgets::mono_value("sudo usermod -aG input $USER"))
                .child(widgets::hint("Sign out and back in after changing groups.")),
        );

        // Onboard profiles and backups
        let has_onboard = self.onboard.data.is_some()
            || self
                .current_device()
                .and_then(|device| device.onboard.as_ref())
                .is_some();
        if has_onboard {
            let writable = self
                .onboard
                .data
                .as_ref()
                .map(|data| data.caps.writable())
                .unwrap_or(false);
            let mut backups = div().flex().flex_col().gap_2();
            if self.settings_ui.show_backups {
                if self.backups.is_empty() {
                    backups = backups.child(widgets::hint(
                        "No backups yet. One is made automatically before the first save.",
                    ));
                }
                for entry in self.backups.clone().into_iter().take(8) {
                    let confirm = self.settings_ui.confirm_restore.as_ref() == Some(&entry.path);
                    let modified = chrono::DateTime::<chrono::Local>::from(entry.modified)
                        .format("%Y-%m-%d %H:%M")
                        .to_string();
                    let path = entry.path.clone();
                    backups = backups.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p(px(10.))
                            .rounded(px(8.))
                            .bg(theme::chrome())
                            .border_1()
                            .border_color(theme::border())
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme::text())
                                            .child(entry.label.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(9.5))
                                            .text_color(theme::dim())
                                            .child(modified),
                                    ),
                            )
                            .child(
                                widgets::ghost_button(if confirm {
                                    "Restore this backup?"
                                } else {
                                    "Restore"
                                })
                                .on_press(cx.listener(
                                    move |this, _, _, cx| {
                                        if this.settings_ui.confirm_restore.as_ref() == Some(&path)
                                        {
                                            this.restore_backup(path.clone(), cx);
                                            this.settings_ui.confirm_restore = None;
                                            this.show_toast(
                                                "Restoring profiles...".into(),
                                                false,
                                                cx,
                                            );
                                        } else {
                                            this.settings_ui.confirm_restore = Some(path.clone());
                                            cx.notify();
                                        }
                                    },
                                )),
                            ),
                    );
                }
            }
            page = page.child(
                widgets::card_padded()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(widgets::heading("Mouse profiles"))
                                    .child(widgets::hint(
                                        "Button maps, DPI stages and lighting in the mouse's own memory. Rusted Snake backs up every profile before its first save.",
                                    )),
                            )
                            .child(widgets::pill(
                                if writable { "WRITES ENABLED" } else { "READ ONLY" },
                                if writable { theme::accent() } else { theme::warning() },
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(widgets::row_label("Experimental saving"))
                                    .child(widgets::hint(
                                        "Allow saving to mouse models that have not been tested. Leave off unless you know your mouse's protocol.",
                                    )),
                            )
                            .child(
                                widgets::switch(self.settings_ui.experimental, theme::accent()).on_press(
                                    cx.listener(|this, _, _, cx| {
                                        let enabled = !this.settings_ui.experimental;
                                        this.set_experimental(enabled, cx);
                                    }),
                                ),
                            ),
                    )
                    .child(widgets::divider())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                widgets::primary_button("Back up now").on_press(cx.listener(
                                    |this, _, _, cx| {
                                        this.backup_now(cx);
                                        this.settings_ui.show_backups = true;
                                    },
                                )),
                            )
                            .child(
                                widgets::ghost_button(if self.settings_ui.show_backups {
                                    "Hide backups"
                                } else {
                                    "Show backups"
                                })
                                .on_press(cx.listener(|this, _, _, cx| {
                                    this.settings_ui.show_backups = !this.settings_ui.show_backups;
                                    if this.settings_ui.show_backups {
                                        this.backend.send(rusted_snake_backend::Command::ListBackups);
                                    }
                                    cx.notify();
                                })),
                            ),
                    )
                    .child(backups)
                    .child(widgets::hint(format!(
                        "Backups live in {}. Install linux/70-rusted-snake-hidraw.rules to allow raw HID access.",
                        rusted_snake_core::paths::backups_dir().display()
                    ))),
            );
        }

        // Startup
        page = page.child(
            widgets::card_padded()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(widgets::heading("Startup"))
                                .child(widgets::hint(
                                    "Open Rusted Snake when you sign in. On X11 the window can also close to the tray, so effects and macros keep running.",
                                )),
                        )
                        .child(
                            widgets::switch(self.settings_ui.autostart, theme::accent()).on_press(cx.listener(
                                |this, _, _, cx| {
                                    let enabled = !this.settings_ui.autostart;
                                    this.set_autostart(enabled, cx);
                                },
                            )),
                        ),
                )
                .child(widgets::info_row(
                    "Autostart entry",
                    rusted_snake_core::paths::autostart_file().display().to_string(),
                )),
        );

        // Macro engine access
        let status = self.macro_ui.status.clone();
        let (color, label) = match status.state {
            EngineState::Running => (theme::accent(), "RUNNING"),
            EngineState::NeedsAccess => (theme::warning(), "NEEDS ACCESS"),
            EngineState::Error => (theme::danger(), "STOPPED"),
            EngineState::Stopped => (theme::dim(), "IDLE"),
        };
        let mut macro_card = widgets::card_padded().gap_3().child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(widgets::heading("Macros"))
                        .child(widgets::hint(status.message.clone())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(widgets::pill(label, color))
                        .child(widgets::ghost_button("Manage").on_press(
                            cx.listener(|this, _, _, cx| this.set_page(Page::Macros, cx)),
                        )),
                ),
        );
        if status.state == EngineState::NeedsAccess {
            macro_card = macro_card
                .child(widgets::divider())
                .child(widgets::hint(
                    "Give Rusted Snake access to the input devices and /dev/uinput:",
                ))
                .child(widgets::mono_value(
                    "sudo cp linux/99-rusted-snake-uinput.rules /etc/udev/rules.d/",
                ))
                .child(widgets::mono_value(
                    "sudo udevadm control --reload-rules && sudo udevadm trigger",
                ))
                .child(widgets::mono_value("sudo usermod -aG input $USER"));
        }
        page = page.child(macro_card);

        // About
        page = page.child(
            widgets::card_padded()
                .gap_3()
                .child(widgets::heading("About Rusted Snake"))
                .child(widgets::hint(format!(
                    "Version {} · a Rust and GPUI rewrite of Snake by Brett Smith.",
                    env!("CARGO_PKG_VERSION")
                )))
                .child(widgets::hint(
                    "Free software under the GNU GPL v3. Razer, Naga and Synapse are trademarks of Razer Inc.; this project is not affiliated with Razer.",
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(widgets::icon("icons/info.svg", 14.0, theme::muted()))
                        .child(widgets::hint(
                            "Settings are in ~/.config/rusted-snake. Device access follows the OpenRazer udev rules.",
                        )),
                ),
        );

        page.into_any_element()
    }
}

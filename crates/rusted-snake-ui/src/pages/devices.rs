use gpui::prelude::*;
use gpui::{AnyElement, Context, FontWeight, MouseButton, Window, div, img, px};

use crate::app::{Page, Root};
use crate::theme;
use crate::widgets::{self, OnPress};

impl Root {
    pub(crate) fn devices_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let device = self.current_device().cloned();
        let mut grid = div().flex().flex_wrap().gap_4().w_full();
        if self.devices.is_empty() {
            grid = grid.child(widgets::empty_state(
                "icons/usb.svg",
                "No Razer devices found",
                "Connect a Razer device and make sure openrazer-daemon is running. Your user must be in the plugdev or openrazer group.",
            ));
        } else {
            for item in self
                .devices
                .clone()
                .into_iter()
                .filter(|item| self.selected.as_deref() != Some(item.id.as_str()))
            {
                let selected = self.selected.as_deref() == Some(item.id.as_str());
                let id = item.id.clone();
                let art = if item.kind == rusted_snake_core::DeviceKind::Mouse {
                    img("art/mouse.png")
                        .w(px(54.))
                        .h(px(76.))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(54.))
                        .rounded(px(12.))
                        .bg(theme::accent_soft(0.08))
                        .child(widgets::icon(item.kind.icon(), 26.0, theme::accent()))
                        .into_any_element()
                };
                let mut card = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(246.))
                    .p(px(18.))
                    .rounded(px(14.))
                    .bg(if selected {
                        theme::surface_raised()
                    } else {
                        theme::surface()
                    })
                    .border_1()
                    .border_color(if selected {
                        theme::accent_soft(0.45)
                    } else {
                        theme::border()
                    })
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(theme::surface_raised())
                            .border_color(theme::border_strong())
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.select_device(id.clone(), cx);
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(art)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap_1()
                                    .child(widgets::pill(item.type_label(), theme::muted()))
                                    .children(item.onboard.as_ref().map(|caps| {
                                        widgets::pill(
                                            if caps.verified { "ONBOARD" } else { "ONBOARD?" },
                                            if caps.verified {
                                                theme::accent()
                                            } else {
                                                theme::warning()
                                            },
                                        )
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_size(px(14.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(item.display_name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(theme::dim())
                                    .child(format!("{:04x}:{:04x}", item.vid, item.pid)),
                            ),
                    );
                if selected {
                    card = card.child(widgets::hint("Open now"));
                }
                grid = grid.child(card);
            }
        }

        let hero = device.as_ref().map(|device| {
            let art = if device.kind == rusted_snake_core::DeviceKind::Mouse {
                img("art/mouse.png")
                    .w(px(132.))
                    .h(px(185.))
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(132.))
                    .h(px(132.))
                    .rounded(px(18.))
                    .bg(theme::accent_soft(0.08))
                    .child(widgets::icon(device.kind.icon(), 58.0, theme::accent()))
                    .into_any_element()
            };
            let battery = device.battery.as_ref();
            let mut stats = div().flex().gap_3().flex_wrap();
            stats = stats.child(widgets::stat(
                "DPI",
                device
                    .dpi
                    .as_ref()
                    .map(|dpi| format!("{} × {}", dpi.x, dpi.y))
                    .unwrap_or_else(|| "—".into()),
                "Current sensor speed",
            ));
            stats = stats.child(widgets::stat(
                "POLLING",
                device
                    .poll_rate
                    .map(|rate| format!("{rate} Hz"))
                    .unwrap_or_else(|| "—".into()),
                "Report rate",
            ));
            stats = stats.child(widgets::stat(
                "BRIGHTNESS",
                device
                    .brightness
                    .map(|value| format!("{value:.0}%"))
                    .unwrap_or_else(|| "—".into()),
                "Lighting level",
            ));
            stats = stats.child(widgets::stat(
                "BATTERY",
                battery
                    .map(|value| format!("{:.0}%", value.percent))
                    .unwrap_or_else(|| "—".into()),
                battery
                    .map(|value| {
                        if value.charging {
                            "Charging"
                        } else {
                            "On battery"
                        }
                    })
                    .unwrap_or("No battery"),
            ));
            widgets::card_padded()
                .w_full()
                .gap_5()
                .child(
                    div().flex().gap_6().child(art).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(widgets::pill("CONNECTED", theme::accent()))
                                    .child(widgets::pill(device.type_label(), theme::muted()))
                                    .children(device.onboard.as_ref().map(|_caps| {
                                        widgets::pill("ONBOARD MEMORY", theme::info())
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(26.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(device.display_name.clone()),
                            )
                            .child(widgets::hint(format!(
                                "Serial {} · Firmware {} · OpenRazer {}",
                                if device.serial.is_empty() {
                                    "unknown"
                                } else {
                                    &device.serial
                                },
                                if device.firmware.is_empty() {
                                    "unknown"
                                } else {
                                    &device.firmware
                                },
                                if self.backend_info.version.is_empty() {
                                    "offline"
                                } else {
                                    &self.backend_info.version
                                }
                            )))
                            .child({
                                let mouse = device.kind == rusted_snake_core::DeviceKind::Mouse;
                                let mut actions = div().flex().items_center().gap_2();
                                if mouse {
                                    actions = actions.child(
                                        widgets::primary_button("Configure buttons").on_press(
                                            cx.listener(|this, _, _, cx| {
                                                this.set_page(Page::Buttons, cx)
                                            }),
                                        ),
                                    );
                                } else {
                                    actions = actions.child(
                                        widgets::primary_button("Lighting").on_press(cx.listener(
                                            |this, _, _, cx| this.set_page(Page::Lighting, cx),
                                        )),
                                    );
                                }
                                if mouse {
                                    actions = actions.child(
                                        widgets::ghost_button("Lighting").on_press(cx.listener(
                                            |this, _, _, cx| this.set_page(Page::Lighting, cx),
                                        )),
                                    );
                                }
                                actions.child(widgets::ghost_button("Performance").on_press(
                                    cx.listener(|this, _, _, cx| {
                                        this.set_page(Page::Performance, cx)
                                    }),
                                ))
                            }),
                    ),
                )
                .child(widgets::divider())
                .child(stats)
                .into_any_element()
        });

        let mut page = div()
            .flex()
            .flex_col()
            .gap_6()
            .p(px(28.))
            .pb(px(44.))
            .max_w(px(1220.))
            .w_full()
            .child(self.page_header(
                "Devices",
                "Choose a device, then tune its buttons, sensor and lighting.",
                "icons/devices.svg",
            ));
        if let Some(hero) = hero {
            page = page.child(hero);
        }
        let other_count = if self.selected.is_some() {
            self.devices.len().saturating_sub(1)
        } else {
            self.devices.len()
        };
        if other_count > 0 {
            page = page
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(widgets::section_label(if self.selected.is_some() {
                            "OTHER DEVICES"
                        } else {
                            "ALL DEVICES"
                        }))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(theme::dim())
                                .child(format!(
                                    "{} device{}",
                                    other_count,
                                    if other_count == 1 { "" } else { "s" }
                                )),
                        ),
                )
                .child(grid);
        }
        page = page
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(theme::chrome())
                    .border_1()
                    .border_color(theme::border())
                    .child(widgets::icon("icons/info.svg", 16.0, theme::info()))
                    .child(widgets::hint(
                        "Rusted Snake speaks to devices through OpenRazer. Lighting and sensor changes are applied live; button changes use the mouse's own memory.",
                    )),
            );
        page.into_any_element()
    }
}

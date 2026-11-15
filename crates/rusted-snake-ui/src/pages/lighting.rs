// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, Window, div, px};

use crate::app::{ColorTarget, LightingSource, Root};
use crate::theme;
use crate::widgets;
use crate::widgets::OnPress;
use rusted_snake_backend::OpenRgbDevice;
use rusted_snake_core::effect::{SoftwareEffect, SoftwareEffectSettings};
use rusted_snake_core::{EffectKind, OnboardLighting, Rgb};

const PRESETS: [Rgb; 6] = [
    Rgb::RAZER_GREEN,
    Rgb::RED,
    Rgb::BLUE,
    Rgb::WHITE,
    Rgb::AMBER,
    Rgb::VIOLET,
];

impl Root {
    pub(crate) fn lighting_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut page = div()
            .flex()
            .flex_col()
            .gap_6()
            .p(px(28.))
            .pb(px(44.))
            .max_w(px(1220.))
            .w_full()
            .child(self.page_header(
                "Lighting",
                "Colours, effects and brightness for every zone.",
                "icons/lighting.svg",
            ))
            .child(self.lighting_tabs(cx));
        if self.lighting.source == LightingSource::OpenRgb {
            return page.child(self.openrgb_lighting(cx)).into_any_element();
        }
        let Some(device) = self.current_device().cloned() else {
            return page
                .child(widgets::empty_state(
                    "icons/lighting.svg",
                    "No device selected",
                    "Pick a Razer device on the Devices page to light it up, or use the OpenRGB tab.",
                ))
                .into_any_element();
        };

        if let Some(brightness) = device.brightness.or(self.lighting.brightness) {
            let preview = self.lighting.brightness.unwrap_or(brightness);
            let weak = cx.entity().downgrade();
            let weak_commit = cx.entity().downgrade();
            page = page.child(
                widgets::card_padded()
                    .gap_4()
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
                                    .child(widgets::heading("Brightness"))
                                    .child(widgets::hint(
                                        "Overall lighting level for this device.",
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(30.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme::accent())
                                            .child(format!("{preview:.0}")),
                                    )
                                    .child(widgets::label("%")),
                            ),
                    )
                    .child(widgets::slider_with_commit(
                        preview as f32,
                        0.0,
                        100.0,
                        1.0,
                        theme::accent(),
                        move |value, _, cx| {
                            let _ = weak.update(cx, |root, cx| {
                                root.lighting.brightness = Some(value as f64);
                                cx.notify();
                            });
                        },
                        move |value, _, cx| {
                            let _ = weak_commit.update(cx, |root, cx| {
                                root.set_brightness(value as f64, cx);
                            });
                        },
                    )),
            );
        }

        let mut zones = div().flex().flex_wrap().gap_4().w_full();
        for zone in device.zones.clone() {
            let interface = zone.id.interface.clone();
            let draft = self
                .lighting
                .zones
                .get(&interface)
                .cloned()
                .unwrap_or_default();
            let mut chips = div().flex().flex_wrap().gap_2();
            for effect in &zone.supported {
                let selected = draft.effect == *effect;
                let effect = *effect;
                let interface = interface.clone();
                let zone_id = zone.id.clone();
                chips = chips.child(widgets::accent_segment(effect.label(), selected).on_press(
                    cx.listener(move |this, _, _, cx| {
                        if let Some(draft) = this.lighting.zones.get_mut(&interface) {
                            draft.effect = effect;
                        }
                        let draft = this
                            .lighting
                            .zones
                            .get(&interface)
                            .cloned()
                            .unwrap_or_default();
                        this.apply_zone_effect(zone_id.clone(), draft, cx);
                    }),
                ));
            }

            let mut body = widgets::card()
                .w(px(350.))
                .p(px(18.))
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(widgets::icon("icons/lighting.svg", 15.0, theme::accent()))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme::text())
                                        .child(zone.id.name.clone()),
                                ),
                        )
                        .child(widgets::pill(
                            zone.brightness
                                .map(|value| format!("{value:.0}%"))
                                .unwrap_or_else(|| "ZONE".into()),
                            theme::muted(),
                        )),
                )
                .child(chips);

            if let Some(brightness) = self
                .lighting
                .zone_brightness
                .get(&zone.id.interface)
                .copied()
            {
                let weak = cx.entity().downgrade();
                let weak_commit = cx.entity().downgrade();
                let interface = zone.id.interface.clone();
                let zone_id = zone.id.clone();
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(widgets::section_label("BRIGHTNESS"))
                                .child(widgets::mono_value(format!("{brightness:.0}%"))),
                        )
                        .child(widgets::slider_with_commit(
                            brightness as f32,
                            0.0,
                            100.0,
                            1.0,
                            theme::accent(),
                            move |value, _, cx| {
                                let _ = weak.update(cx, |root, cx| {
                                    root.lighting
                                        .zone_brightness
                                        .insert(interface.clone(), value as f64);
                                    cx.notify();
                                });
                            },
                            move |value, _, cx| {
                                let _ = weak_commit.update(cx, |root, cx| {
                                    root.set_zone_brightness(zone_id.clone(), value as f64, cx);
                                });
                            },
                        )),
                );
            }

            if draft.effect.needs_color() {
                let mut row = div().flex().items_center().gap_2().flex_wrap();
                row = row.child(widgets::color_preview(draft.colors[0]).size(px(26.)));
                for preset in PRESETS {
                    let interface = interface.clone();
                    let zone_id = zone.id.clone();
                    row = row.child(
                        widgets::swatch(preset, draft.colors[0].distance(preset) < 6.0).on_press(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(draft) = this.lighting.zones.get_mut(&interface) {
                                    draft.colors[0] = preset;
                                }
                                let draft = this
                                    .lighting
                                    .zones
                                    .get(&interface)
                                    .cloned()
                                    .unwrap_or_default();
                                this.apply_zone_effect(zone_id.clone(), draft, cx);
                            }),
                        ),
                    );
                }
                let interface = interface.clone();
                let color = draft.colors[0];
                row = row.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.open_color_picker(ColorTarget::Zone(interface.clone(), 0), color, cx);
                    }),
                ));
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("COLOUR"))
                        .child(row),
                );
            }

            if draft.effect.needs_two_colors() {
                let mut row = div().flex().items_center().gap_2().flex_wrap();
                row = row.child(widgets::color_preview(draft.colors[1]).size(px(26.)));
                for preset in PRESETS {
                    let interface = interface.clone();
                    let zone_id = zone.id.clone();
                    row = row.child(
                        widgets::swatch(preset, draft.colors[1].distance(preset) < 6.0).on_press(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(draft) = this.lighting.zones.get_mut(&interface) {
                                    draft.colors[1] = preset;
                                }
                                let draft = this
                                    .lighting
                                    .zones
                                    .get(&interface)
                                    .cloned()
                                    .unwrap_or_default();
                                this.apply_zone_effect(zone_id.clone(), draft, cx);
                            }),
                        ),
                    );
                }
                let interface = interface.clone();
                let color = draft.colors[1];
                row = row.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.open_color_picker(ColorTarget::Zone(interface.clone(), 1), color, cx);
                    }),
                ));
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("SECOND COLOUR"))
                        .child(row),
                );
            }

            if draft.effect.needs_speed() {
                let weak = cx.entity().downgrade();
                let weak_commit = cx.entity().downgrade();
                let interface_speed = interface.clone();
                let interface_speed_commit = interface.clone();
                let zone_speed = zone.id.clone();
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(widgets::section_label("SPEED"))
                                .child(widgets::mono_value(format!("{}", draft.speed))),
                        )
                        .child(widgets::slider_with_commit(
                            draft.speed as f32,
                            1.0,
                            255.0,
                            1.0,
                            theme::accent(),
                            move |value, _, cx| {
                                let _ = weak.update(cx, |root, cx| {
                                    if let Some(draft) =
                                        root.lighting.zones.get_mut(&interface_speed)
                                    {
                                        draft.speed = value.round() as u8;
                                    }
                                    cx.notify();
                                });
                            },
                            move |_, _, cx| {
                                let _ = weak_commit.update(cx, |root, cx| {
                                    let draft = root
                                        .lighting
                                        .zones
                                        .get(&interface_speed_commit)
                                        .cloned()
                                        .unwrap_or_default();
                                    root.apply_zone_effect(zone_speed.clone(), draft, cx);
                                });
                            },
                        )),
                );
            }

            if draft.effect == EffectKind::Wave {
                let mut row = div().flex().items_center().gap_2();
                for (direction, label) in [(0, "Backward"), (1, "Forward")] {
                    let interface = interface.clone();
                    let zone_id = zone.id.clone();
                    row = row.child(
                        widgets::segment(label, draft.direction == direction).on_press(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(draft) = this.lighting.zones.get_mut(&interface) {
                                    draft.direction = direction;
                                }
                                let draft = this
                                    .lighting
                                    .zones
                                    .get(&interface)
                                    .cloned()
                                    .unwrap_or_default();
                                this.apply_zone_effect(zone_id.clone(), draft, cx);
                            }),
                        ),
                    );
                }
                body = body.child(row);
            }

            body = body.child(widgets::hint(draft.effect.description()));
            zones = zones.child(body);
        }

        if !device.zones.is_empty() {
            page = page.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::section_label("ZONES"))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(theme::dim())
                            .child(format!(
                                "{} zone{}",
                                device.zones.len(),
                                if device.zones.len() == 1 { "" } else { "s" }
                            )),
                    ),
            );
            page = page.child(zones);
        }

        page = page.child(self.software_effects_card(&device, cx));
        page = page.child(self.onboard_lighting_card(cx));
        page.into_any_element()
    }

    fn lighting_tabs(&self, cx: &mut Context<Self>) -> Div {
        let mut bar = div()
            .flex()
            .items_center()
            .gap_1()
            .p(px(4.))
            .rounded(px(11.))
            .bg(theme::chrome())
            .border_1()
            .border_color(theme::border());
        for (source, label) in [
            (LightingSource::OpenRazer, "OPENRAZER"),
            (LightingSource::OpenRgb, "OPENRGB"),
        ] {
            let active = self.lighting.source == source;
            bar = bar.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(34.))
                    .px(px(18.))
                    .rounded(px(8.))
                    .bg(if active {
                        theme::surface_raised()
                    } else {
                        gpui::transparent_black()
                    })
                    .border_1()
                    .border_color(if active {
                        theme::border()
                    } else {
                        gpui::transparent_black()
                    })
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if active {
                        theme::accent()
                    } else {
                        theme::muted()
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::surface()).text_color(theme::text()))
                    .on_press(cx.listener(move |this, _, _, cx| {
                        this.open_lighting_source(source, cx);
                    }))
                    .child(widgets::dot(if active {
                        theme::accent()
                    } else {
                        theme::dim()
                    }))
                    .child(label),
            );
        }
        bar
    }

    fn openrgb_lighting(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div().flex().flex_col().gap_5().w_full();
        let header = div()
            .flex()
            .items_start()
            .justify_between()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(widgets::heading("OpenRGB devices"))
                    .child(widgets::hint(
                        "OpenRGB covers devices OpenRazer does not manage, including motherboards, RAM and some Razer gear. Changes use the openrgb command.",
                    )),
            )
            .child(
                widgets::ghost_button("Refresh").on_press(cx.listener(|this, _, _, cx| {
                    this.refresh_openrgb(cx);
                })),
            );
        body = body.child(header);

        if self.lighting.openrgb_loading {
            return body
                .child(
                    widgets::card_padded()
                        .items_center()
                        .gap_3()
                        .child(widgets::icon("icons/refresh.svg", 18.0, theme::accent()))
                        .child(widgets::hint("Asking OpenRGB for devices...")),
                )
                .into_any_element();
        }
        if let Some(error) = &self.lighting.openrgb_error {
            return body
                .child(
                    widgets::card_padded()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(widgets::icon("icons/alert.svg", 16.0, theme::warning()))
                                .child(widgets::heading("OpenRGB is not available")),
                        )
                        .child(widgets::hint(error.clone()))
                        .child(widgets::hint(
                            "Install OpenRGB and make sure the openrgb command can see your hardware. This tab uses the command line rather than linking OpenRGB.",
                        )),
                )
                .into_any_element();
        }
        if self.lighting.openrgb_devices.is_empty() {
            return body
                .child(widgets::empty_state(
                    "icons/lighting.svg",
                    "No OpenRGB devices",
                    "OpenRGB returned no lighting controllers. Check that OpenRGB can see your hardware, then refresh.",
                ))
                .into_any_element();
        }

        let mut cards = div().flex().flex_wrap().gap_3().w_full();
        for device in self.lighting.openrgb_devices.clone() {
            let selected = self.lighting.openrgb_selected == Some(device.index);
            let index = device.index;
            cards = cards.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w(px(258.))
                    .p(px(14.))
                    .rounded(px(11.))
                    .bg(if selected {
                        theme::surface_raised()
                    } else {
                        theme::surface()
                    })
                    .border_1()
                    .border_color(if selected {
                        theme::accent_soft(0.5)
                    } else {
                        theme::border()
                    })
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(theme::surface_raised())
                            .border_color(theme::border_strong())
                    })
                    .on_press(cx.listener(move |this, _, _, cx| {
                        this.select_openrgb_device(Some(index), cx);
                    }))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text())
                            .child(device.name.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::pill(device.kind.clone(), theme::muted()))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme::dim())
                                    .child(format!("#{}", device.index)),
                            ),
                    )
                    .child(widgets::hint(device.description.clone())),
            );
        }
        body = body.child(cards);

        let Some(device) = self.openrgb_selected_device().cloned() else {
            return body
                .child(widgets::hint("Choose an OpenRGB device."))
                .into_any_element();
        };
        let mut mode_chips = div().flex().flex_wrap().gap_2();
        for mode in &device.modes {
            let selected = self.lighting.openrgb_mode.as_deref() == Some(mode.as_str());
            let mode = mode.clone();
            mode_chips =
                mode_chips.child(widgets::accent_segment(mode.clone(), selected).on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.set_openrgb_mode(mode.clone(), cx);
                    }),
                ));
        }
        let mut zone_chips = div().flex().flex_wrap().gap_2();
        zone_chips = zone_chips.child(
            widgets::segment("All zones", self.lighting.openrgb_zone.is_none())
                .on_press(cx.listener(|this, _, _, cx| this.set_openrgb_zone(None, cx))),
        );
        for (index, zone) in device.zones.iter().enumerate() {
            let selected = self.lighting.openrgb_zone == Some(index);
            let name = zone.clone();
            zone_chips = zone_chips.child(widgets::segment(name, selected).on_press(
                cx.listener(move |this, _, _, cx| this.set_openrgb_zone(Some(index), cx)),
            ));
        }
        let mut colours = div().flex().items_center().gap_2().flex_wrap();
        colours = colours.child(widgets::color_preview(self.lighting.openrgb_color).size(px(28.)));
        for preset in PRESETS {
            let selected = self.lighting.openrgb_color.distance(preset) < 6.0;
            colours = colours.child(widgets::swatch(preset, selected).on_press(cx.listener(
                move |this, _, _, cx| {
                    this.lighting.openrgb_color = preset;
                    this.apply_openrgb(cx);
                },
            )));
        }
        colours = colours.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
            cx.listener(move |this, _, _, cx| {
                this.open_color_picker(ColorTarget::OpenRgb, this.lighting.openrgb_color, cx);
            }),
        ));

        let brightness_weak = cx.entity().downgrade();
        let brightness_commit = cx.entity().downgrade();
        let speed_weak = cx.entity().downgrade();
        let speed_commit = cx.entity().downgrade();

        body = body.child(
            widgets::card_padded()
                .gap_5()
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
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme::text())
                                        .child(device.name.clone()),
                                )
                                .child(widgets::hint(format!(
                                    "{} · {} · {}",
                                    device.kind,
                                    device.description,
                                    if device.location.is_empty() { "OpenRGB" } else { &device.location }
                                ))),
                        )
                        .child(widgets::pill("OPENRGB", theme::info())),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("MODE"))
                        .child(mode_chips),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("ZONE"))
                        .child(zone_chips),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("COLOUR"))
                        .child(colours),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(widgets::section_label("BRIGHTNESS"))
                                .child(widgets::mono_value(format!("{:.0}%", self.lighting.openrgb_brightness))),
                        )
                        .child(widgets::slider_with_commit(
                            self.lighting.openrgb_brightness,
                            0.0,
                            100.0,
                            1.0,
                            theme::accent(),
                            move |value, _, cx| {
                                let _ = brightness_weak.update(cx, |root, cx| {
                                    root.lighting.openrgb_brightness = value;
                                    cx.notify();
                                });
                            },
                            move |_, _, cx| {
                                let _ = brightness_commit.update(cx, |root, cx| root.apply_openrgb(cx));
                            },
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(widgets::section_label("SPEED"))
                                .child(widgets::mono_value(format!("{:.0}%", self.lighting.openrgb_speed))),
                        )
                        .child(widgets::slider_with_commit(
                            self.lighting.openrgb_speed,
                            0.0,
                            100.0,
                            1.0,
                            theme::accent(),
                            move |value, _, cx| {
                                let _ = speed_weak.update(cx, |root, cx| {
                                    root.lighting.openrgb_speed = value;
                                    cx.notify();
                                });
                            },
                            move |_, _, cx| {
                                let _ = speed_commit.update(cx, |root, cx| root.apply_openrgb(cx));
                            },
                        )),
                )
                .child(widgets::hint(
                    "OpenRGB and OpenRazer can control the same device. Prefer one backend at a time so the last write wins predictably.",
                )),
        );
        body.into_any_element()
    }

    fn openrgb_selected_device(&self) -> Option<&OpenRgbDevice> {
        let index = self.lighting.openrgb_selected?;
        self.lighting
            .openrgb_devices
            .iter()
            .find(|device| device.index == index)
    }

    fn software_effects_card(
        &mut self,
        device: &rusted_snake_core::DeviceSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = self.settings.software.clone();
        let active = self.software_active.get(&device.id).copied();
        let can_static = device
            .zones
            .iter()
            .any(|zone| zone.supported.contains(&EffectKind::Static));
        if !can_static {
            return widgets::card_padded()
                .gap_3()
                .child(widgets::heading("Live effects"))
                .child(widgets::hint("This device does not accept static colours through OpenRazer, so software effects are not available."))
                .into_any_element();
        }

        let mut chips = div().flex().flex_wrap().gap_2();
        for (effect, label) in [
            (SoftwareEffect::None, "Saved lighting"),
            (SoftwareEffect::Spectrum, "Spectrum"),
            (SoftwareEffect::Breath, "Breath"),
            (SoftwareEffect::Audio, "Audio"),
        ] {
            let selected =
                active == Some(effect) || (effect == SoftwareEffect::None && active.is_none());
            chips = chips.child(
                widgets::accent_segment(label, selected).on_press(cx.listener(
                    move |this, _, _, cx| {
                        let mut next = this.settings.software.clone();
                        next.effect = effect;
                        if effect == SoftwareEffect::None {
                            this.apply_software_effect(next, cx);
                            this.reapply_onboard_lighting(cx);
                        } else {
                            this.apply_software_effect(next, cx);
                        }
                    },
                )),
            );
        }

        let mut body = widgets::card_padded()
            .gap_5()
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
                            .child(widgets::heading("Live effects"))
                            .child(widgets::hint(
                                "Effects run while Rusted Snake is open. Pick Saved lighting to return to the profile's own colours.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .children(active.map(|effect| {
                                widgets::pill(format!("{} RUNNING", effect.label().to_uppercase()), theme::accent())
                            }))
                            .child(widgets::icon("icons/sliders.svg", 16.0, theme::muted())),
                    ),
            )
            .child(chips);

        match settings.effect {
            SoftwareEffect::Spectrum => {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(widgets::section_label("SPEED"))
                        .child(self.software_speed_slider(settings.clone(), cx)),
                );
            }
            SoftwareEffect::Breath => {
                let mut row = div().flex().items_center().gap_2().flex_wrap();
                row = row.child(widgets::color_preview(settings.colors[0]).size(px(28.)));
                for preset in PRESETS {
                    let index = 0;
                    row = row.child(
                        widgets::swatch(preset, settings.colors[0].distance(preset) < 6.0)
                            .on_press(cx.listener(move |this, _, _, cx| {
                                if let Some(slot) = this.settings.software.colors.get_mut(index) {
                                    *slot = preset;
                                }
                                let next = this.settings.software.clone();
                                this.apply_software_effect(next, cx);
                            })),
                    );
                }
                let color = settings.colors[0];
                row = row.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.open_color_picker(ColorTarget::SoftwareColor(0), color, cx);
                    }),
                ));
                let mut row2 = div().flex().items_center().gap_2().flex_wrap();
                let second = settings.colors.get(1).copied().unwrap_or(Rgb::RAZER_GREEN);
                row2 = row2.child(widgets::color_preview(second).size(px(28.)));
                for preset in PRESETS {
                    row2 = row2.child(
                        widgets::swatch(preset, second.distance(preset) < 6.0).on_press(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(slot) = this.settings.software.colors.get_mut(1) {
                                    *slot = preset;
                                }
                                let next = this.settings.software.clone();
                                this.apply_software_effect(next, cx);
                            }),
                        ),
                    );
                }
                row2 = row2.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.open_color_picker(ColorTarget::SoftwareColor(1), second, cx);
                    }),
                ));
                body = body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(widgets::section_label("FIRST COLOUR"))
                            .child(row),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(widgets::section_label("SECOND COLOUR"))
                            .child(row2),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(widgets::section_label("SPEED"))
                            .child(self.software_speed_slider(settings.clone(), cx)),
                    );
            }
            SoftwareEffect::Audio => {
                let mut sources = div().flex().flex_wrap().gap_2();
                sources = sources.child(
                    widgets::segment("System default", settings.audio_source.is_none()).on_press(
                        cx.listener(|this, _, _, cx| {
                            this.settings.software.audio_source = None;
                            let next = this.settings.software.clone();
                            this.apply_software_effect(next, cx);
                        }),
                    ),
                );
                for source in self.audio_sources.clone().into_iter().take(8) {
                    let selected = settings.audio_source.as_deref() == Some(source.id.as_str());
                    let id = source.id.clone();
                    let label = short_source_label(&source.label);
                    sources =
                        sources.child(widgets::segment(label, selected).max_w(px(240.)).on_press(
                            cx.listener(move |this, _, _, cx| {
                                this.settings.software.audio_source = Some(id.clone());
                                let next = this.settings.software.clone();
                                this.apply_software_effect(next, cx);
                            }),
                        ));
                }
                let mut colours = div().flex().items_center().gap_3().flex_wrap();
                for index in 0..3 {
                    let preset_color = settings
                        .audio_colors
                        .get(index)
                        .copied()
                        .unwrap_or(Rgb::RAZER_GREEN);
                    let weak = cx.entity().downgrade();
                    colours = colours.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(widgets::section_label(match index {
                                0 => "QUIET",
                                1 => "MIDDLE",
                                _ => "LOUD",
                            }))
                            .child(widgets::swatch(preset_color, false).on_press(cx.listener(
                                move |this, _, _, cx| {
                                    this.open_color_picker(
                                        ColorTarget::AudioColor(index),
                                        preset_color,
                                        cx,
                                    );
                                },
                            ))),
                    );
                    let _ = weak;
                }
                body = body
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(widgets::section_label("LISTEN TO"))
                                    .child(widgets::hint(if self.audio_sources.is_empty() {
                                        "No sources found. Check PipeWire or PulseAudio."
                                    } else {
                                        "System default follows what is playing on your default output. Pick a monitor source to be explicit."
                                    })),
                            )
                            .child(sources),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(widgets::section_label("SENSITIVITY"))
                                    .child(widgets::mono_value(format!("{:.1}×", settings.audio_gain))),
                            )
                            .child(self.audio_gain_slider(settings.clone(), cx)),
                    )
                    .child(div().flex().items_center().gap_3().child(widgets::section_label("COLOURS")).child(colours));
            }
            SoftwareEffect::None => {
                body = body.child(widgets::hint(
                    "Saved lighting is showing. Choose an effect above to animate the lights while Rusted Snake is open.",
                ));
            }
        }

        body.into_any_element()
    }

    fn software_speed_slider(
        &self,
        settings: SoftwareEffectSettings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        let weak_commit = cx.entity().downgrade();
        widgets::slider_with_commit(
            settings.speed * 100.0,
            0.0,
            100.0,
            1.0,
            theme::accent(),
            move |value, _, cx| {
                let _ = weak.update(cx, |root, cx| {
                    root.settings.software.speed = value / 100.0;
                    cx.notify();
                });
            },
            move |value, _, cx| {
                let _ = weak_commit.update(cx, |root, cx| {
                    root.settings.software.speed = value / 100.0;
                    let next = root.settings.software.clone();
                    root.apply_software_effect(next, cx);
                });
            },
        )
        .into_any_element()
    }

    fn audio_gain_slider(
        &self,
        settings: SoftwareEffectSettings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        let weak_commit = cx.entity().downgrade();
        widgets::slider_with_commit(
            settings.audio_gain,
            0.2,
            8.0,
            0.1,
            theme::accent(),
            move |value, _, cx| {
                let _ = weak.update(cx, |root, cx| {
                    root.settings.software.audio_gain = value;
                    cx.notify();
                });
            },
            move |value, _, cx| {
                let _ = weak_commit.update(cx, |root, cx| {
                    root.settings.software.audio_gain = value;
                    let next = root.settings.software.clone();
                    root.apply_software_effect(next, cx);
                });
            },
        )
        .into_any_element()
    }

    fn onboard_lighting_card(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(data) = self.onboard.data.clone() else {
            return div().into_any_element();
        };
        let writable = data.caps.writable();
        let mut profile_chips = div().flex().items_center().gap_2();
        for slot in 1..=data.caps.profile_count {
            let selected = self.onboard.profile == slot;
            profile_chips =
                profile_chips.child(widgets::segment(format!("P{slot}"), selected).on_press(
                    cx.listener(move |this, _, _, cx| this.onboard_profile_changed(slot, cx)),
                ));
        }
        let mut zones = div().flex().flex_wrap().gap_4();
        for zone in data.zones.clone() {
            let lighting = self
                .onboard
                .lights
                .get(&zone.id)
                .cloned()
                .unwrap_or_else(OnboardLighting::off);
            let mut chips = div().flex().gap_2();
            for (effect, label) in [
                (OnboardLighting::OFF, "Off"),
                (OnboardLighting::STATIC, "Static"),
                (OnboardLighting::SPECTRUM, "Spectrum"),
            ] {
                let selected = lighting.effect == effect;
                chips = chips.child(widgets::segment(label, selected).on_press(cx.listener(
                    move |this, _, _, cx| {
                        this.set_onboard_lighting_effect(zone.id, effect, cx);
                    },
                )));
            }
            let mut body = widgets::card()
                .w(px(340.))
                .p(px(16.))
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme::text())
                                .child(zone.name.clone()),
                        )
                        .child(widgets::hint(lighting.describe())),
                )
                .child(chips);
            if lighting.effect == OnboardLighting::STATIC {
                let current = lighting
                    .colors
                    .first()
                    .copied()
                    .map(Rgb::from_led)
                    .unwrap_or(Rgb::RAZER_GREEN);
                let mut row = div().flex().items_center().gap_2().flex_wrap();
                row = row.child(widgets::color_preview(current).size(px(26.)));
                for preset in PRESETS {
                    let zone_id = zone.id;
                    row = row.child(
                        widgets::swatch(preset, current.distance(preset) < 6.0).on_press(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(lighting) = this.onboard.lights.get_mut(&zone_id) {
                                    lighting.colors = vec![preset.to_led()];
                                }
                                cx.notify();
                            }),
                        ),
                    );
                }
                let zone_id = zone.id;
                row = row.child(widgets::tiny_icon_button("icons/palette.svg").on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.open_color_picker(ColorTarget::Onboard(zone_id, 0), current, cx);
                    }),
                ));
                body = body.child(row);
            }
            zones = zones.child(body);
        }
        let mut save = widgets::primary_button("Save lighting to mouse");
        if writable {
            save = save.on_press(cx.listener(|this, _, _, cx| {
                this.save_onboard_lighting(cx);
                this.show_toast("Saving lighting...".into(), false, cx);
            }));
        } else {
            save = save.opacity(0.45);
        }
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
                            .child(widgets::heading("Lighting saved on the mouse"))
                            .child(widgets::hint(
                                "These colours live in the mouse's own profile, so they work without Rusted Snake running.",
                            )),
                    )
                    .child(profile_chips),
            )
            .child(zones)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::hint(if writable {
                        "Changes apply to the selected onboard profile."
                    } else {
                        "Saving to this mouse is off. Turn on experimental saving in Settings."
                    }))
                    .child(save),
            )
            .into_any_element()
    }
}

fn short_source_label(label: &str) -> String {
    let label = label
        .strip_prefix("Monitor of ")
        .map(|rest| format!("Monitor · {rest}"))
        .unwrap_or_else(|| label.to_string());
    if label.chars().count() > 34 {
        let mut shortened: String = label.chars().take(31).collect();
        shortened.push_str("...");
        shortened
    } else {
        label
    }
}

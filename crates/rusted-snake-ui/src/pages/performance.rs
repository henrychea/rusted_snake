// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use gpui::prelude::*;
use gpui::{AnyElement, Context, FontWeight, Window, div, px};

use crate::app::Root;
use crate::theme;
use crate::widgets;
use crate::widgets::OnPress;

impl Root {
    pub(crate) fn performance_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(device) = self.current_device().cloned() else {
            return div()
                .p(px(28.))
                .child(widgets::empty_state(
                    "icons/performance.svg",
                    "No device selected",
                    "Pick a Razer device on the Devices page to tune its sensor.",
                ))
                .into_any_element();
        };
        let mut page = div()
            .flex()
            .flex_col()
            .gap_6()
            .p(px(28.))
            .pb(px(44.))
            .max_w(px(1220.))
            .w_full()
            .child(self.page_header(
                "Performance",
                "Tune the sensor, DPI stages and report rate.",
                "icons/performance.svg",
            ));

        let max_dpi = device
            .dpi
            .as_ref()
            .map(|dpi| dpi.max)
            .unwrap_or(16000)
            .max(1600);
        let onboard = self.onboard.data.clone();

        if let Some(data) = onboard {
            let profile = data
                .profile(self.onboard.profile)
                .cloned()
                .unwrap_or_default();
            let mut stage_rows = div().flex().flex_col().gap_3();
            for (index, value) in self.performance.stages.iter().copied().enumerate() {
                let selected = self.performance.active_stage == index;
                let weak = cx.entity().downgrade();
                let mut label = widgets::segment(
                    if selected {
                        format!("Start · Stage {}", index + 1)
                    } else {
                        format!("Stage {}", index + 1)
                    },
                    selected,
                );
                label = label
                    .w(px(122.))
                    .on_press(cx.listener(move |this, _, _, cx| {
                        this.performance.active_stage = index;
                        cx.notify();
                    }));
                stage_rows = stage_rows.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_4()
                        .child(label)
                        .child(
                            widgets::slider_with_commit(
                                value as f32,
                                100.0,
                                max_dpi as f32,
                                50.0,
                                theme::accent(),
                                move |new_value, _, cx| {
                                    let _ = weak.update(cx, |root, cx| {
                                        if let Some(stage) = root.performance.stages.get_mut(index)
                                        {
                                            *stage = new_value.round() as u16;
                                        }
                                        cx.notify();
                                    });
                                },
                                |_, _, _| {},
                            )
                            .flex_1(),
                        )
                        .child(
                            div()
                                .w(px(92.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme::text())
                                .child(format!("{} DPI", value)),
                        ),
                );
            }
            let count = self.performance.stages.len();
            let mut stage_actions = div().flex().items_center().gap_2();
            if count < 5 {
                stage_actions = stage_actions.child(widgets::ghost_button("Add stage").on_press(
                    cx.listener(|this, _, _, cx| {
                        let value = this
                            .performance
                            .stages
                            .last()
                            .copied()
                            .unwrap_or(1600)
                            .saturating_add(800)
                            .min(16000);
                        this.performance.stages.push(value);
                        cx.notify();
                    }),
                ));
            }
            if count > 1 {
                stage_actions = stage_actions.child(widgets::ghost_button("Remove last").on_press(
                    cx.listener(|this, _, _, cx| {
                        if this.performance.stages.len() > 1 {
                            this.performance.stages.pop();
                            this.performance.active_stage = this
                                .performance
                                .active_stage
                                .min(this.performance.stages.len().saturating_sub(1));
                        }
                        cx.notify();
                    }),
                ));
            }
            let writable = data.caps.writable();
            let mut save = widgets::primary_button("Save stages to mouse");
            if writable {
                save = save.on_press(cx.listener(|this, _, _, cx| {
                    this.save_onboard_stages(cx);
                    this.show_toast("Saving DPI stages...".into(), false, cx);
                }));
            } else {
                save = save.opacity(0.45);
            }

            page = page.child(
                widgets::card_padded()
                    .gap_5()
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
                                    .child(widgets::heading("DPI stages"))
                                    .child(widgets::hint(
                                        "The DPI buttons on top of the mouse step through these speeds. The highlighted stage is used when this profile loads.",
                                    )),
                            )
                            .child(widgets::pill(
                                format!("PROFILE {}", self.onboard.profile),
                                theme::accent(),
                            )),
                    )
                    .child(stage_rows)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(stage_actions)
                            .child(save),
                    )
                    .child(widgets::hint(format!(
                        "Stored in the mouse itself. Profile {} currently starts at stage {} ({} DPI).",
                        profile.slot,
                        self.performance.active_stage + 1,
                        profile.dpi.dpi.get(self.performance.active_stage).copied().unwrap_or(0)
                    ))),
            );
        } else if let Some(dpi) = device.dpi.clone() {
            let preview = self.performance.dpi.unwrap_or(dpi.x);
            let weak_change = cx.entity().downgrade();
            let weak_commit = cx.entity().downgrade();
            page = page.child(
                widgets::card_padded()
                    .gap_5()
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(widgets::heading("Sensor sensitivity"))
                                    .child(widgets::hint(
                                        "Changes are applied live through OpenRazer.",
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(38.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme::accent())
                                            .child(format!("{preview}")),
                                    )
                                    .child(widgets::label("DPI")),
                            ),
                    )
                    .child(widgets::slider_with_commit(
                        preview as f32,
                        100.0,
                        dpi.max as f32,
                        50.0,
                        theme::accent(),
                        move |value, _, cx| {
                            let _ = weak_change.update(cx, |root, cx| {
                                root.performance.dpi = Some(value.round() as i32);
                                cx.notify();
                            });
                        },
                        move |value, _, cx| {
                            let _ = weak_commit.update(cx, |root, cx| {
                                root.set_dpi(value.round() as i32, cx);
                            });
                        },
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(widgets::hint(format!(
                                "{} to {} DPI · step 50",
                                100, dpi.max
                            )))
                            .child(widgets::mono_value(format!("X {} · Y {}", dpi.x, dpi.y))),
                    ),
            );
        } else {
            page = page.child(widgets::empty_state(
                "icons/performance.svg",
                "No sensor controls",
                "This device does not expose DPI control through OpenRazer.",
            ));
        }

        if let Some(rate) = self.performance.poll_rate.or(device.poll_rate) {
            let options = [125u32, 500, 1000, 2000, 4000, 8000];
            let mut chips = div().flex().items_center().gap_2();
            for option in options {
                let selected = option == rate;
                chips =
                    chips.child(widgets::segment(format!("{option} Hz"), selected).on_press(
                        cx.listener(move |this, _, _, cx| this.set_poll_rate(option, cx)),
                    ));
            }
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
                                    .child(widgets::heading("Polling rate"))
                                    .child(widgets::hint(
                                        "How often the mouse reports its position. Higher is smoother and uses more CPU.",
                                    )),
                            )
                            .child(widgets::pill(format!("{rate} HZ"), theme::accent())),
                    )
                    .child(chips)
                    .child(widgets::hint("1000 Hz suits most people. Use 4000 or 8000 Hz only if your computer keeps up.")),
            );
        }

        page.into_any_element()
    }
}

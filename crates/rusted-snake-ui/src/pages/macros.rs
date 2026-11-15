// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, Window, div, px};

use crate::app::{CaptureTarget, FieldId, MacroEditor, MacroKind, Root};
use crate::theme;
use crate::widgets;
use crate::widgets::OnPress;
use rusted_snake_backend::EngineState;
use rusted_snake_core::keymap;
use rusted_snake_core::macros::{MacroAction, MacroDef, RepeatMode};

impl Root {
    pub(crate) fn macros_page(
        &mut self,
        window: &mut Window,
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
                "Macros",
                "A button press can type text, press another key or run a command. Macros run while Rusted Snake is open.",
                "icons/macros.svg",
            ));
        page = page.child(self.engine_card(cx));
        page = page.child(self.set_row(window, cx));

        if self.selected.is_none() {
            return page
                .child(widgets::empty_state(
                    "icons/macros.svg",
                    "No device selected",
                    "Pick a Razer device on the Devices page, then add macros that its side buttons can run.",
                ))
                .into_any_element();
        }

        let device = self.selected.clone().unwrap_or_default();
        let mut entries: Vec<MacroDef> = self
            .macros
            .active()
            .macros
            .iter()
            .filter(|entry| entry.device.as_deref().is_none_or(|value| value == device))
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.trigger.unwrap_or(u16::MAX));

        let mut list = div().flex().flex_col().gap_2().w_full();
        if entries.is_empty() {
            list = list.child(
                widgets::card_padded()
                    .items_center()
                    .gap_3()
                    .child(widgets::icon("icons/macros.svg", 26.0, theme::dim()))
                    .child(widgets::hint("No macros for this device yet."))
                    .child(widgets::primary_button("Add your first macro").on_press(
                        cx.listener(|this, _, window, cx| this.open_macro_editor(None, window, cx)),
                    )),
            );
        }
        for entry in entries {
            list = list.child(self.macro_row(entry, cx));
        }

        let main = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::section_label("MACROS FOR THIS DEVICE"))
                    .child(widgets::primary_button("Add macro").on_press(
                        cx.listener(|this, _, window, cx| this.open_macro_editor(None, window, cx)),
                    )),
            )
            .child(list);

        page = page.child(match self.macro_ui.editor.is_some() {
            true => div()
                .flex()
                .gap_5()
                .items_start()
                .w_full()
                .child(div().flex_1().min_w_0().child(main))
                .child(self.macro_editor(window, cx))
                .into_any_element(),
            false => main.into_any_element(),
        });
        page.into_any_element()
    }

    fn engine_card(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let status = self.macro_ui.status.clone();
        let (color, label) = match status.state {
            EngineState::Running => (theme::accent(), "RUNNING"),
            EngineState::NeedsAccess => (theme::warning(), "NEEDS ACCESS"),
            EngineState::Error => (theme::danger(), "STOPPED"),
            EngineState::Stopped => (theme::dim(), "IDLE"),
        };
        let mut card = widgets::card_padded()
            .gap_3()
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
                            .child(widgets::heading("Macro engine"))
                            .child(widgets::hint(
                                "Macros watch the Razer input devices and type through a virtual keyboard. They only run while Rusted Snake is open.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::pill(label, color))
                            .child(
                                widgets::icon_button("icons/refresh.svg").on_press(cx.listener(
                                    |this, _, _, cx| {
                                        this.reload_macro_engine();
                                        this.show_toast("Refreshing macro engine...".into(), false, cx);
                                    },
                                )),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(widgets::dot(color))
                    .child(widgets::hint(status.message.clone())),
            );
        if !status.devices.is_empty() {
            card = card.child(widgets::hint(format!(
                "Input devices: {}",
                status.devices.join(", ")
            )));
        }
        if status.state == EngineState::NeedsAccess {
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p(px(12.))
                    .rounded(px(9.))
                    .bg(theme::chrome())
                    .border_1()
                    .border_color(theme::border())
                    .child(widgets::section_label("ENABLE MACROS"))
                    .child(widgets::hint("Add yourself to the input group and install the included udev rule, then sign in again:"))
                    .child(widgets::mono_value("sudo cp linux/99-rusted-snake-uinput.rules /etc/udev/rules.d/"))
                    .child(widgets::mono_value("sudo udevadm control --reload-rules && sudo udevadm trigger"))
                    .child(widgets::mono_value("sudo usermod -aG input $USER")),
            );
        }
        card.into_any_element()
    }

    fn set_row(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let sets = self.macros.sets.clone();
        let active = self.macros.active_set;
        let mut chips = div().flex().items_center().gap_2().flex_wrap();
        for (index, set) in sets.iter().enumerate() {
            let selected = index == active;
            chips = chips.child(widgets::segment(set.name.clone(), selected).on_press(
                cx.listener(move |this, _, _, cx| {
                    this.macros.set_active(index);
                    this.save_macros(cx);
                }),
            ));
        }
        chips = chips.child(
            widgets::tiny_icon_button("icons/plus.svg").on_press(cx.listener(|this, _, _, cx| {
                let mut number = this.macros.sets.len() + 1;
                while this
                    .macros
                    .sets
                    .iter()
                    .any(|set| set.name == format!("Set {number}"))
                {
                    number += 1;
                }
                this.macros.add_set(format!("Set {number}"));
                this.save_macros(cx);
            })),
        );

        let mut actions = div().flex().items_center().gap_2();
        actions = actions.child(widgets::ghost_button("Rename").on_press(cx.listener(
            |this, _, _, cx| {
                let name = this.macros.active().name.clone();
                this.fields.entry(FieldId::SetName).or_default().set(name);
                this.macro_ui.renaming_set = true;
                cx.notify();
            },
        )));
        if self.macros.sets.len() > 1 {
            actions = actions.child(
                widgets::danger_button(if self.macro_ui.confirm_delete_set {
                    "Confirm delete"
                } else {
                    "Delete set"
                })
                .on_press(cx.listener(|this, _, _, cx| {
                    if this.macro_ui.confirm_delete_set {
                        this.macros.remove_active_set();
                        this.macro_ui.confirm_delete_set = false;
                        this.save_macros(cx);
                    } else {
                        this.macro_ui.confirm_delete_set = true;
                        cx.notify();
                    }
                })),
            );
        }

        let mut body = div().flex().flex_col().gap_3();
        if self.macro_ui.renaming_set {
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(300.)).child(self.field_element(
                        FieldId::SetName,
                        "Set name",
                        false,
                        window,
                        cx,
                    )))
                    .child(widgets::primary_button("Save").on_press(cx.listener(
                        |this, _, _, cx| {
                            let name = this
                                .fields
                                .get(&FieldId::SetName)
                                .map(|field| field.value.trim().to_string())
                                .unwrap_or_default();
                            if !name.is_empty() {
                                this.macros.active_mut().name = name;
                                this.save_macros(cx);
                            }
                            this.macro_ui.renaming_set = false;
                            cx.notify();
                        },
                    )))
                    .child(
                        widgets::tiny_icon_button("icons/close.svg").on_press(cx.listener(
                            |this, _, _, cx| {
                                this.macro_ui.renaming_set = false;
                                cx.notify();
                            },
                        )),
                    ),
            );
        }
        body = body.child(chips);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p(px(14.))
            .rounded(px(11.))
            .bg(theme::surface())
            .border_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::section_label("MACRO SETS"))
                    .child(actions),
            )
            .child(body)
            .into_any_element()
    }

    fn macro_row(&self, entry: MacroDef, cx: &mut Context<Self>) -> impl IntoElement {
        let id = entry.id;
        let confirm = self.macro_ui.confirm_delete_macro == Some(id);
        div()
            .flex()
            .items_center()
            .gap_4()
            .p(px(13.))
            .rounded(px(10.))
            .bg(theme::surface())
            .border_1()
            .border_color(theme::border())
            .cursor_pointer()
            .hover(|style| {
                style
                    .bg(theme::surface_raised())
                    .border_color(theme::border_strong())
            })
            .on_press(cx.listener(move |this, _, window, cx| {
                this.open_macro_editor(Some(id), window, cx);
            }))
            .child(widgets::keycap(entry.trigger_label()).min_w(px(118.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::text())
                            .child(if entry.name.is_empty() {
                                entry.action.kind_label().to_string()
                            } else {
                                entry.name.clone()
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .child(format!(
                                "{} · {}",
                                entry.action.describe(),
                                entry.repeat.label()
                            )),
                    ),
            )
            .child(widgets::pill(
                entry.action.kind_label().to_uppercase(),
                theme::muted(),
            ))
            .child(
                widgets::tiny_icon_button(if confirm {
                    "icons/alert.svg"
                } else {
                    "icons/trash.svg"
                })
                .on_press(cx.listener(move |this, _, _, cx| {
                    if this.macro_ui.confirm_delete_macro == Some(id) {
                        this.delete_macro(id, cx);
                        this.macro_ui.confirm_delete_macro = None;
                    } else {
                        this.macro_ui.confirm_delete_macro = Some(id);
                        cx.notify();
                    }
                })),
            )
    }

    fn macro_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let editor = self.macro_ui.editor.as_ref().unwrap();
        let editing = editor.id.is_some();
        let kind = editor.kind;
        let repeat = editor.repeat;
        let trigger = editor.trigger;
        let key = editor.key;
        let key_modifiers = editor.key_modifiers;

        let mut kind_chips = div().flex().flex_wrap().gap_2();
        for option in [MacroKind::Key, MacroKind::Text, MacroKind::Command] {
            let selected = kind == option;
            kind_chips =
                kind_chips.child(widgets::accent_segment(option.label(), selected).on_press(
                    cx.listener(move |this, _, _, cx| {
                        if let Some(editor) = &mut this.macro_ui.editor {
                            editor.kind = option;
                        }
                        cx.notify();
                    }),
                ));
        }

        let detail = match kind {
            MacroKind::Key => {
                let label = if key.is_some_and(|code| code > 0) {
                    format!(
                        "{}{}",
                        key_modifiers.label(),
                        keymap::evdev_name(key.unwrap_or(0))
                    )
                } else {
                    "Click, then press a key".to_string()
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(widgets::section_label("KEY TO PRESS"))
                    .child(self.capture_element(CaptureTarget::MacroKey, label, window, cx))
                    .into_any_element()
            }
            MacroKind::Text => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(widgets::section_label("TEXT TO TYPE"))
                .child(self.field_element(
                    FieldId::MacroText,
                    "Hello from Rusted Snake",
                    false,
                    window,
                    cx,
                ))
                .child(widgets::hint(
                    "ASCII text is typed. Each character is sent through the virtual keyboard.",
                ))
                .into_any_element(),
            MacroKind::Command => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(widgets::section_label("COMMAND"))
                .child(self.field_element(
                    FieldId::MacroCommand,
                    "notify-send 'Hello'",
                    true,
                    window,
                    cx,
                ))
                .child(widgets::hint(
                    "Runs with /bin/sh while Rusted Snake is open.",
                ))
                .into_any_element(),
        };

        let mut trigger_chips = div().flex().flex_wrap().gap_2();
        for (code, label) in keymap::SIDE_TRIGGERS {
            let selected = trigger == Some(*code);
            let code = *code;
            trigger_chips = trigger_chips.child(
                widgets::segment(
                    format!("S{}", label.rsplit(' ').next().unwrap_or("")),
                    selected,
                )
                .on_press(cx.listener(move |this, _, _, cx| {
                    if let Some(editor) = &mut this.macro_ui.editor {
                        editor.trigger = Some(code);
                    }
                    cx.notify();
                })),
            );
        }
        let trigger_label = trigger
            .map(|code| format!("Custom: {}", keymap::evdev_name(code)))
            .unwrap_or_else(|| "Press a key to use it as the trigger".into());

        let mut repeat_chips = div().flex().flex_wrap().gap_2();
        for option in [RepeatMode::Once, RepeatMode::WhileHeld, RepeatMode::Toggle] {
            let selected = repeat == option;
            repeat_chips = repeat_chips.child(widgets::segment(option.label(), selected).on_press(
                cx.listener(move |this, _, _, cx| {
                    if let Some(editor) = &mut this.macro_ui.editor {
                        editor.repeat = option;
                    }
                    cx.notify();
                }),
            ));
        }

        div()
            .flex()
            .flex_col()
            .gap_4()
            .w(px(400.))
            .p(px(18.))
            .rounded(px(14.))
            .bg(theme::surface())
            .border_1()
            .border_color(theme::accent_soft(0.25))
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
                            .child(widgets::section_label(if editing {
                                "EDIT MACRO"
                            } else {
                                "NEW MACRO"
                            }))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child("Macro action"),
                            ),
                    )
                    .child(
                        widgets::tiny_icon_button("icons/close.svg").on_press(cx.listener(
                            |this, _, _, cx| {
                                this.macro_ui.editor = None;
                                this.capture = None;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(widgets::section_label("NAME"))
                    .child(self.field_element(
                        FieldId::MacroName,
                        "Optional name",
                        false,
                        window,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(widgets::section_label("WHAT IT DOES"))
                    .child(kind_chips),
            )
            .child(detail)
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
                            .child(widgets::section_label("RUNS WHEN"))
                            .child(widgets::hint("Side plate buttons")),
                    )
                    .child(trigger_chips)
                    .child(self.capture_element(
                        CaptureTarget::MacroTrigger,
                        trigger_label,
                        window,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(widgets::section_label("REPEAT"))
                    .child(repeat_chips),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(widgets::ghost_button("Cancel").on_press(cx.listener(
                        |this, _, _, cx| {
                            this.macro_ui.editor = None;
                            this.capture = None;
                            cx.notify();
                        },
                    )))
                    .child(
                        widgets::primary_button(if editing {
                            "Save changes"
                        } else {
                            "Create macro"
                        })
                        .on_press(cx.listener(|this, _, _, cx| this.save_macro_editor(cx))),
                    ),
            )
    }
}

#[allow(dead_code)]
fn _unused(_: MacroAction, _: MacroEditor) {}

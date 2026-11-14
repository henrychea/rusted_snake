use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, MouseButton, Window, div, img, px};

use crate::app::{ActionKind, CaptureTarget, Page, Root};
use crate::theme;
use crate::widgets::{self, OnPress};
use rusted_snake_core::{DeviceKind, OnboardAction, OnboardButton, OnboardData};

impl Root {
    pub(crate) fn buttons_page(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(device) = self.current_device().cloned() else {
            return div()
                .p(px(28.))
                .child(widgets::empty_state(
                    "icons/mouse.svg",
                    "No device selected",
                    "Pick a Razer device on the Devices page to set up its buttons.",
                ))
                .into_any_element();
        };

        let header = self.page_header(
            "Buttons",
            "Click a button to change what it does. Changes are saved in the mouse's own memory.",
            "icons/buttons.svg",
        );
        let mut page = div()
            .flex()
            .flex_col()
            .gap_6()
            .p(px(28.))
            .max_w(px(1220.))
            .w_full()
            .child(header);

        if let Some(error) = self.onboard.error.clone() {
            page = page.child(
                widgets::card_padded()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::icon("icons/alert.svg", 16.0, theme::warning()))
                            .child(widgets::heading("Onboard profiles unavailable")),
                    )
                    .child(widgets::hint(error))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(widgets::ghost_button("Try again").on_press(cx.listener(
                                |this, _, _, cx| {
                                    this.request_onboard();
                                    this.show_toast("Reading the mouse...".into(), false, cx);
                                },
                            )))
                            .children((device.kind == DeviceKind::Mouse).then(|| {
                                widgets::ghost_button("Allow access in Settings").on_press(
                                    cx.listener(|this, _, _, cx| this.set_page(Page::Settings, cx)),
                                )
                            })),
                    ),
            );
        }

        if self.onboard.loading {
            page = page.child(
                widgets::card_padded()
                    .items_center()
                    .gap_3()
                    .child(widgets::logo(28.0))
                    .child(widgets::hint("Reading profiles from the mouse...")),
            );
        }

        let Some(data) = self.onboard.data.clone() else {
            if self.onboard.error.is_none() && !self.onboard.loading {
                page = page.child(
                    widgets::empty_state(
                        "icons/buttons.svg",
                        "No onboard buttons for this device",
                        "Button remapping uses the memory inside Razer mice. This device does not expose onboard profiles.",
                    ),
                );
            }
            return page.into_any_element();
        };

        page = page.child(self.profile_strip(&data, cx));

        let plates = plate_groups(&data);
        if !plates.is_empty() {
            let mut strip = div().flex().items_center().gap_2();
            strip = strip.child(widgets::section_label("SIDE PLATE"));
            for plate in &plates {
                let selected = self.onboard.plate.as_deref() == Some(plate.as_str());
                let plate_id = plate.clone();
                strip = strip.child(widgets::segment(plate_label(plate), selected).on_press(
                    cx.listener(move |this, _, _, cx| {
                        this.onboard.plate = Some(plate_id.clone());
                        this.onboard.editing = None;
                        if let Some(device) = this.selected.clone() {
                            this.settings.onboard_plate.insert(device, plate_id.clone());
                            let _ = this.settings.save();
                        }
                        cx.notify();
                    }),
                ));
            }
            page = page.child(strip);
        }

        let side_buttons: Vec<OnboardButton> = self
            .onboard
            .plate
            .as_ref()
            .map(|plate| {
                data.buttons
                    .iter()
                    .filter(|button| &button.group == plate)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let mut side = div().flex().flex_col().gap_2().w(px(218.));
        side = side.child(widgets::section_label(
            self.onboard
                .plate
                .as_deref()
                .map(plate_label)
                .unwrap_or_else(|| "SIDE BUTTONS".into()),
        ));
        let mut side_grid = div().flex().flex_wrap().gap_2();
        for button in &side_buttons {
            side_grid = side_grid.child(div().w(px(64.)).child(self.button_tile(button, cx)));
        }
        if side_buttons.is_empty() {
            side = side.child(widgets::hint("No side buttons on this plate."));
        } else {
            side = side.child(side_grid);
        }

        let mut top_groups: Vec<(String, Vec<OnboardButton>)> = Vec::new();
        for button in &data.buttons {
            if plate_groups(&data).contains(&button.group) {
                continue;
            }
            if let Some((_, buttons)) = top_groups
                .iter_mut()
                .find(|(group, _)| group == &button.group)
            {
                buttons.push(button.clone());
            } else {
                top_groups.push((button.group.clone(), vec![button.clone()]));
            }
        }
        let mut right = div().flex().flex_col().gap_4().w(px(236.));
        for (group, buttons) in &top_groups {
            let mut column = div().flex().flex_col().gap_2();
            column = column.child(widgets::section_label(group.to_uppercase()));
            for button in buttons {
                column = column.child(self.button_tile(button, cx));
            }
            right = right.child(column);
        }

        let art = div()
            .relative()
            .w(px(230.))
            .h(px(322.))
            .child(img("art/mouse.png").absolute().inset_0().w_full().h_full());
        let art = art.children(
            data.buttons
                .iter()
                .filter_map(|button| hotspot(button.id).map(|(x, y)| (button.id, x, y)))
                .map(|(id, x, y)| {
                    let selected = self.onboard.editing == Some(id);
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(20.))
                        .rounded_full()
                        .bg(if selected {
                            theme::accent()
                        } else {
                            theme::with_alpha(0x000000, 0.72)
                        })
                        .border_1()
                        .border_color(if selected {
                            theme::accent()
                        } else {
                            theme::accent_soft(0.7)
                        })
                        .text_size(px(9.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if selected {
                            theme::on_accent()
                        } else {
                            theme::accent()
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::accent_soft(0.9)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.open_button_editor(id, cx);
                            }),
                        )
                        .child(hotspot_label(id))
                        .into_any_element()
                }),
        );

        let stage = div()
            .flex()
            .items_center()
            .justify_center()
            .gap_6()
            .w_full()
            .p(px(18.))
            .rounded(px(14.))
            .bg(theme::chrome())
            .border_1()
            .border_color(theme::border())
            .child(side)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(art)
                    .child(widgets::hint("Click a marker or a button")),
            )
            .child(right);

        page = page.child(stage).child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .p(px(14.))
                .rounded(px(11.))
                .bg(theme::chrome())
                .border_1()
                .border_color(theme::border())
                .child(widgets::icon("icons/save.svg", 15.0, theme::accent()))
                .child(widgets::hint(format!(
                    "{} · all profiles are backed up before the first save.",
                    if data.caps.writable() {
                        "Saving is enabled for this mouse"
                    } else {
                        "Experimental saving is off for this mouse"
                    }
                ))),
        );
        page.into_any_element()
    }

    fn profile_strip(&self, data: &OnboardData, cx: &mut Context<Self>) -> Div {
        let strip = div()
            .flex()
            .items_center()
            .justify_between()
            .p(px(14.))
            .rounded(px(12.))
            .bg(theme::surface())
            .border_1()
            .border_color(theme::border());
        let mut left = div().flex().items_center().gap_3();
        left = left
            .child(widgets::section_label("ONBOARD PROFILE"))
            .child(widgets::hint(format!(
                "{} of {} slots · active slot {}",
                data.profiles_in_use.len(),
                data.caps.profile_count,
                data.active_profile
            )));
        let mut chips = div().flex().items_center().gap_2();
        for slot in 1..=data.caps.profile_count {
            let selected = slot == self.onboard.profile;
            let in_use = data.profiles_in_use.contains(&slot);
            let mut chip = widgets::segment(format!("P{slot}"), selected);
            if !in_use {
                chip = chip.opacity(0.5);
            }
            chips = chips.child(chip.on_press(cx.listener(move |this, _, _, cx| {
                this.onboard_profile_changed(slot, cx);
            })));
        }
        strip.child(left).child(chips)
    }

    fn button_tile(&self, button: &OnboardButton, cx: &mut Context<Self>) -> impl IntoElement {
        let action = self
            .onboard
            .data
            .as_ref()
            .and_then(|data| data.profile(self.onboard.profile))
            .and_then(|profile| profile.buttons.get(&button.id))
            .cloned();
        let selected = self.onboard.editing == Some(button.id);
        let id = button.id;
        let name = if button.group == "Bottom" {
            "Underside button".to_string()
        } else {
            button.name.clone()
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p(px(11.))
            .rounded(px(9.))
            .bg(if selected {
                theme::surface_hover()
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
                    .bg(theme::surface_hover())
                    .border_color(theme::border_strong())
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.open_button_editor(id, cx);
                }),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::text())
                    .child(name),
            )
            .child(
                div().text_size(px(10.5)).text_color(theme::muted()).child(
                    action
                        .map(|action| action.describe())
                        .unwrap_or_else(|| "Unread".into()),
                ),
            )
    }

    pub fn open_button_editor(&mut self, id: u8, cx: &mut Context<Self>) {
        let current = self
            .onboard
            .data
            .as_ref()
            .and_then(|data| data.profile(self.onboard.profile))
            .and_then(|profile| profile.buttons.get(&id))
            .cloned()
            .unwrap_or_else(OnboardAction::disabled);
        self.onboard.pending_action = Some(current.clone());
        self.onboard.action_kind = ActionKind::from_action(&current);
        if current.action_type == OnboardAction::MOUSE {
            self.onboard.mouse_button = current.params.first().copied().unwrap_or(4);
        }
        self.onboard.editing = Some(id);
        self.capture = None;
        cx.notify();
    }

    pub(crate) fn button_drawer(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let id = self.onboard.editing?;
        Some(
            div()
                .absolute()
                .top(px(56.))
                .right(px(0.))
                .bottom(px(0.))
                .w(px(380.))
                .p(px(16.))
                .bg(theme::surface())
                .border_l_1()
                .border_color(theme::border())
                .shadow_lg()
                .child(self.button_editor(id, window, cx))
                .into_any_element(),
        )
    }

    fn button_editor(&mut self, id: u8, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let data = self.onboard.data.clone().unwrap_or_default();
        let button = data.buttons.iter().find(|button| button.id == id).cloned();
        let name = button
            .as_ref()
            .map(|button| {
                if button.group == "Bottom" {
                    "Underside button".to_string()
                } else if button.group.contains("plate") {
                    format!("{}, {}", button.group, button.name)
                } else {
                    button.name.clone()
                }
            })
            .unwrap_or_else(|| format!("Button {id:02x}"));
        let current = self.onboard.pending_action.clone().unwrap_or_default();
        let writable = data.caps.writable();
        let mut kinds = vec![
            ActionKind::Key,
            ActionKind::Mouse,
            ActionKind::DpiUp,
            ActionKind::DpiDown,
            ActionKind::NextProfile,
            ActionKind::Disabled,
        ];
        if matches!(
            current.action_type,
            OnboardAction::MACRO | OnboardAction::MACRO_REPEAT | OnboardAction::SCROLL
        ) {
            kinds.insert(0, ActionKind::Keep);
        }
        let mut kind_grid = div().flex().flex_wrap().gap_2();
        for kind in kinds {
            let selected = self.onboard.action_kind == kind;
            kind_grid = kind_grid.child(
                widgets::accent_segment(kind.label(), selected)
                    .w(px(132.))
                    .on_press(cx.listener(move |this, _, window, cx| {
                        this.choose_action_kind(kind, window, cx);
                    })),
            );
        }
        let mut detail = div().flex().flex_col().gap_3();
        match self.onboard.action_kind {
            ActionKind::Key => {
                let label = self
                    .onboard
                    .pending_action
                    .as_ref()
                    .filter(|action| action.action_type == OnboardAction::KEY)
                    .map(|action| action.describe())
                    .unwrap_or_else(|| "Click here, then press a key".into());
                detail = detail.child(self.capture_element(
                    CaptureTarget::OnboardKey,
                    label,
                    window,
                    cx,
                ));
                detail = detail.child(widgets::hint(
                    "Hold Ctrl, Shift, Alt or Super while pressing a key to add it.",
                ));
            }
            ActionKind::Mouse => {
                let mut row = div().flex().flex_wrap().gap_2();
                for (value, label) in [
                    (1u8, "Left"),
                    (2, "Right"),
                    (3, "Middle"),
                    (4, "Back"),
                    (5, "Forward"),
                    (9, "Scroll left"),
                    (10, "Scroll right"),
                ] {
                    let selected = self.onboard.mouse_button == value;
                    row = row.child(widgets::segment(label, selected).on_press(cx.listener(
                        move |this, _, _, cx| {
                            this.onboard.mouse_button = value;
                            this.onboard.pending_action = Some(OnboardAction::mouse(value));
                            cx.notify();
                        },
                    )));
                }
                detail = detail.child(row);
            }
            ActionKind::Keep => {
                detail = detail.child(widgets::hint(
                    "The macro or scroll action saved by Synapse will be left alone.",
                ));
            }
            ActionKind::DpiUp
            | ActionKind::DpiDown
            | ActionKind::NextProfile
            | ActionKind::Disabled => {
                detail = detail.child(widgets::hint(format!(
                    "This button will {}",
                    self.onboard.action_kind.label().to_lowercase()
                )));
            }
        }

        let special_left = id == 0x01;
        let save_enabled = writable && !special_left;
        let mut save = widgets::primary_button("Save to mouse");
        if !save_enabled {
            save = save.opacity(0.45).cursor(gpui::CursorStyle::Arrow);
        } else {
            save = save.on_press(cx.listener(|this, _, _, cx| this.save_onboard_button(cx)));
        }

        div()
            .flex()
            .flex_col()
            .gap_4()
            .w_full()
            .p(px(18.))
            .rounded(px(14.))
            .bg(theme::surface())
            .border_1()
            .border_color(theme::accent_soft(0.25))
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
                            .child(widgets::section_label("EDIT BUTTON"))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(name),
                            ),
                    )
                    .child(widgets::tiny_icon_button("icons/close.svg").on_press(cx.listener(
                        |this, _, _, cx| {
                            this.onboard.editing = None;
                            this.capture = None;
                            cx.notify();
                        },
                    ))),
            )
            .child(widgets::info_row("Now", current.describe()))
            .child(if special_left {
                widgets::hint("Left click cannot be changed, so the mouse always stays usable.")
            } else {
                widgets::hint("Choose what this button should do.")
            })
            .child(kind_grid)
            .child(detail)
            .child(if !writable {
                widgets::hint("Saving to this mouse has not been tested. Turn on experimental saving in Settings.")
            } else {
                widgets::hint("Saved in the profile inside the mouse.")
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(widgets::ghost_button("Cancel").on_press(cx.listener(
                        |this, _, _, cx| {
                            this.onboard.editing = None;
                            this.capture = None;
                            cx.notify();
                        },
                    )))
                    .child(save),
            )
    }

    fn choose_action_kind(
        &mut self,
        kind: ActionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.onboard.action_kind = kind;
        match kind {
            ActionKind::Keep => {}
            ActionKind::Key => {
                self.capture = Some(CaptureTarget::OnboardKey);
                window.focus(&self.capture_focus);
            }
            ActionKind::Mouse => {
                self.onboard.pending_action = Some(OnboardAction::mouse(self.onboard.mouse_button));
            }
            ActionKind::DpiUp => self.onboard.pending_action = Some(OnboardAction::dpi_up()),
            ActionKind::DpiDown => self.onboard.pending_action = Some(OnboardAction::dpi_down()),
            ActionKind::NextProfile => {
                self.onboard.pending_action = Some(OnboardAction::next_profile())
            }
            ActionKind::Disabled => self.onboard.pending_action = Some(OnboardAction::disabled()),
        }
        cx.notify();
    }
}

fn plate_groups(data: &OnboardData) -> Vec<String> {
    let mut plates: Vec<String> = data
        .buttons
        .iter()
        .map(|button| button.group.clone())
        .filter(|group| group.contains("plate"))
        .collect();
    plates.sort();
    plates.dedup();
    plates
}

fn plate_label(plate: &str) -> String {
    plate.replace("-button plate", " buttons").to_uppercase()
}

fn hotspot_label(id: u8) -> &'static str {
    match id {
        0x01 => "L",
        0x02 => "R",
        0x03 => "M",
        0x34 => "U",
        0x35 => "D",
        0x0b => "+",
        0x0c => "-",
        0x0e => "P",
        _ => "•",
    }
}

fn hotspot(id: u8) -> Option<(f32, f32)> {
    Some(match id {
        0x01 => (68.0, 62.0),
        0x02 => (162.0, 62.0),
        0x03 => (110.0, 108.0),
        0x34 => (110.0, 88.0),
        0x35 => (110.0, 128.0),
        0x0b => (72.0, 172.0),
        0x0c => (158.0, 172.0),
        0x0e => (110.0, 292.0),
        _ => return None,
    })
}

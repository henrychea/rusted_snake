use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, BorderStyle, Bounds, CursorStyle, Div, ElementId, Hsla, MouseButton, Pixels,
    SharedString, Window, canvas, div, point, px, quad, size, svg, transparent_black,
};

use crate::theme;

pub trait OnPress: Sized {
    fn on_press(
        self,
        listener: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> Self;
}

impl OnPress for Div {
    fn on_press(
        self,
        listener: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_mouse_down(MouseButton::Left, listener)
    }
}

pub fn icon(path: &'static str, size: f32, color: Hsla) -> impl IntoElement {
    svg().path(path).size(px(size)).text_color(color)
}

pub fn logo(size: f32) -> impl IntoElement {
    svg()
        .path("icons/logo.svg")
        .size(px(size))
        .text_color(theme::accent())
}

pub fn card() -> Div {
    div()
        .flex()
        .flex_col()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border())
        .rounded(px(14.))
}

pub fn card_padded() -> Div {
    card().p(px(22.))
}

pub fn section_label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::dim())
        .child(text.into())
}

pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(18.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .child(text.into())
}

pub fn hint(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.5))
        .line_height(px(19.))
        .text_color(theme::muted())
        .child(text.into())
}

pub fn label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme::muted())
        .child(text.into())
}

pub fn value(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(14.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .child(text.into())
}

pub fn primary_button(text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(38.))
        .px(px(18.))
        .rounded(px(9.))
        .bg(theme::accent())
        .text_color(theme::on_accent())
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|style| style.bg(gpui::rgb(0x5ae83f)))
        .child(text.into())
}

pub fn ghost_button(text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(38.))
        .px(px(15.))
        .rounded(px(9.))
        .bg(theme::surface_raised())
        .border_1()
        .border_color(theme::border())
        .text_color(theme::text())
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(theme::surface_hover())
                .border_color(theme::border_strong())
        })
        .child(text.into())
}

pub fn danger_button(text: impl Into<SharedString>) -> Div {
    ghost_button(text).text_color(theme::danger())
}

pub fn icon_button(path: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(36.))
        .rounded(px(9.))
        .bg(theme::surface_raised())
        .border_1()
        .border_color(theme::border())
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(theme::surface_hover())
                .border_color(theme::border_strong())
        })
        .child(icon(path, 17.0, theme::muted()))
}

pub fn tiny_icon_button(path: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded(px(7.))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .child(icon(path, 14.0, theme::muted()))
}

pub fn pill(text: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(23.))
        .px(px(9.))
        .rounded_full()
        .bg(theme::with_alpha(0xffffff, 0.04))
        .border_1()
        .border_color(theme::with_alpha(0xffffff, 0.06))
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(color)
        .child(text.into())
}

pub fn dot(color: Hsla) -> Div {
    div().size(px(7.)).rounded_full().bg(color)
}

pub fn switch(on: bool, color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .w(px(42.))
        .h(px(23.))
        .p(px(3.))
        .rounded_full()
        .bg(if on { color } else { theme::surface_hover() })
        .border_1()
        .border_color(if on { color } else { theme::border_strong() })
        .cursor_pointer()
        .justify_end()
        .when(!on, |element| element.justify_start())
        .child(div().size(px(15.)).rounded_full().bg(if on {
            theme::on_accent()
        } else {
            theme::muted()
        }))
}

pub fn segment(text: impl Into<SharedString>, selected: bool) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.))
        .px(px(13.))
        .rounded(px(7.))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if selected {
            theme::text()
        } else {
            theme::muted()
        })
        .bg(if selected {
            theme::surface_hover()
        } else {
            theme::surface_raised()
        })
        .border_1()
        .border_color(if selected {
            theme::border_strong()
        } else {
            theme::border()
        })
        .cursor_pointer()
        .hover(|style| style.border_color(theme::border_strong()))
        .child(text.into())
}

pub fn accent_segment(text: impl Into<SharedString>, selected: bool) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.))
        .px(px(13.))
        .rounded(px(7.))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(if selected {
            theme::on_accent()
        } else {
            theme::muted()
        })
        .bg(if selected {
            theme::accent()
        } else {
            theme::surface_raised()
        })
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::border()
        })
        .cursor_pointer()
        .child(text.into())
}

pub fn divider() -> Div {
    div().h(px(1.)).w_full().bg(theme::border())
}

pub fn swatch(color: rusted_snake_core::Rgb, selected: bool) -> Div {
    let hsla = theme::from_rgb(color);
    div()
        .size(px(24.))
        .rounded_full()
        .bg(hsla)
        .border_1()
        .border_color(if selected {
            theme::text()
        } else {
            theme::with_alpha(0xffffff, 0.16)
        })
        .cursor_pointer()
        .hover(|style| style.border_color(theme::with_alpha(0xffffff, 0.55)))
}

pub fn empty_state(
    icon_path: &'static str,
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .p(px(48.))
        .w_full()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border())
        .rounded(px(14.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(54.))
                .rounded(px(15.))
                .bg(theme::accent_soft(0.10))
                .border_1()
                .border_color(theme::accent_soft(0.22))
                .child(icon(icon_path, 24.0, theme::accent())),
        )
        .child(
            div()
                .text_size(px(16.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::text())
                .child(title.into()),
        )
        .child(
            div()
                .max_w(px(420.))
                .text_align(gpui::TextAlign::Center)
                .text_size(px(12.5))
                .line_height(px(19.))
                .text_color(theme::muted())
                .child(body.into()),
        )
}

pub fn stat(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    sub: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p(px(16.))
        .rounded(px(11.))
        .bg(theme::chrome())
        .border_1()
        .border_color(theme::border())
        .child(section_label(label))
        .child(
            div()
                .text_size(px(19.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::text())
                .child(value.into()),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::dim())
                .child(sub.into()),
        )
}

pub fn info_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py(px(9.))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(theme::muted())
                .child(label.into()),
        )
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme::text())
                .child(value.into()),
        )
}

pub fn badge(title: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px(px(10.))
        .py(px(6.))
        .rounded(px(8.))
        .bg(theme::with_alpha(0xffffff, 0.035))
        .border_1()
        .border_color(theme::border())
        .child(dot(color))
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::muted())
                .child(title.into()),
        )
}

pub fn keycap(text: impl Into<SharedString>) -> Div {
    div()
        .min_w(px(32.))
        .h(px(28.))
        .px(px(8.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .bg(theme::surface_raised())
        .border_1()
        .border_color(theme::border_strong())
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme::text())
        .child(text.into())
}

pub struct TextField<'a> {
    pub value: &'a str,
    pub placeholder: &'a str,
    pub caret: usize,
    pub focused: bool,
    pub mono: bool,
    pub height: f32,
}

pub fn text_field(field: TextField<'_>) -> Div {
    let caret = field.caret.min(field.value.len());
    let before = field.value.get(..caret).unwrap_or(field.value);
    let after = field.value.get(caret..).unwrap_or("");
    let empty = field.value.is_empty();
    let mut element = div()
        .flex()
        .items_center()
        .h(px(field.height))
        .w_full()
        .px(px(12.))
        .rounded(px(8.))
        .bg(theme::chrome())
        .border_1()
        .border_color(if field.focused {
            theme::accent_soft(0.6)
        } else {
            theme::border()
        })
        .text_size(px(13.))
        .text_color(theme::text())
        .cursor(CursorStyle::IBeam)
        .overflow_hidden();
    if field.mono {
        element = element.font_family("monospace");
    }
    if empty {
        if field.focused {
            element = element
                .child(div().w(px(1.5)).h(px(18.)).mx(px(0.5)).bg(theme::accent()))
                .child(
                    div()
                        .text_color(theme::dim())
                        .child(field.placeholder.to_string()),
                );
        } else {
            element = element.child(
                div()
                    .text_color(theme::dim())
                    .child(field.placeholder.to_string()),
            );
        }
    } else {
        element = element
            .child(div().child(before.to_string()))
            .child(
                div()
                    .w(px(1.5))
                    .h(px(18.))
                    .mx(px(0.5))
                    .bg(if field.focused {
                        theme::accent()
                    } else {
                        theme::dim()
                    }),
            )
            .child(div().child(after.to_string()));
    }
    element
}

pub fn slider(
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    color: Hsla,
    on_change: impl Fn(f32, &mut Window, &mut gpui::App) + 'static,
) -> Div {
    slider_with_commit(value, min, max, step, color, on_change, |_, _, _| {})
}

pub fn slider_with_commit(
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    color: Hsla,
    on_change: impl Fn(f32, &mut Window, &mut gpui::App) + 'static,
    on_commit: impl Fn(f32, &mut Window, &mut gpui::App) + 'static,
) -> Div {
    let bounds = Rc::new(Cell::new(Bounds::<Pixels> {
        origin: point(px(0.), px(0.)),
        size: size(px(0.), px(0.)),
    }));
    let dragging = Rc::new(Cell::new(false));
    let last = Rc::new(Cell::new(value.clamp(min, max)));
    let callback = Rc::new(on_change);
    let commit = Rc::new(on_commit);
    let span = (max - min).max(f32::EPSILON);
    let fraction = ((value - min) / span).clamp(0.0, 1.0);
    let bounds_for_paint = bounds.clone();
    let bounds_for_down = bounds.clone();
    let bounds_for_move = bounds.clone();
    let dragging_down = dragging.clone();
    let dragging_move = dragging.clone();
    let dragging_up = dragging.clone();
    let dragging_up_out = dragging.clone();
    let callback_down = callback.clone();
    let callback_move = callback.clone();
    let last_down = last.clone();
    let last_move = last.clone();
    let last_up = last.clone();
    let last_up_out = last.clone();
    let commit_up = commit.clone();
    let commit_up_out = commit.clone();

    div()
        .h(px(28.))
        .w_full()
        .relative()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            dragging_down.set(true);
            let bounds = bounds_for_down.get();
            if f32::from(bounds.size.width) <= 0.0 {
                return;
            }
            let x = (event.position.x - bounds.origin.x) / bounds.size.width;
            let raw = min + x.clamp(0.0, 1.0) * span;
            let stepped = if step > 0.0 {
                (raw / step).round() * step
            } else {
                raw
            };
            let stepped = stepped.clamp(min, max);
            last_down.set(stepped);
            callback_down(stepped, window, cx);
        })
        .on_mouse_move(move |event, window, cx| {
            if !dragging_move.get() {
                return;
            }
            let bounds = bounds_for_move.get();
            if f32::from(bounds.size.width) <= 0.0 {
                return;
            }
            let x = (event.position.x - bounds.origin.x) / bounds.size.width;
            let raw = min + x.clamp(0.0, 1.0) * span;
            let stepped = if step > 0.0 {
                (raw / step).round() * step
            } else {
                raw
            };
            let stepped = stepped.clamp(min, max);
            last_move.set(stepped);
            callback_move(stepped, window, cx);
        })
        .on_mouse_up(MouseButton::Left, move |_, window, cx| {
            if dragging_up.replace(false) {
                commit_up(last_up.get(), window, cx);
            }
        })
        .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
            if dragging_up_out.replace(false) {
                commit_up_out(last_up_out.get(), window, cx);
            }
        })
        .child(canvas(
            move |bounds, _, _| {
                bounds_for_paint.set(bounds);
                bounds
            },
            move |bounds, _, window, _| {
                let track_height = px(6.);
                let track = Bounds {
                    origin: point(
                        bounds.origin.x,
                        bounds.origin.y + (bounds.size.height - track_height) * 0.5,
                    ),
                    size: size(bounds.size.width, track_height),
                };
                window.paint_quad(quad(
                    track,
                    px(3.),
                    theme::surface_hover(),
                    px(0.),
                    transparent_black(),
                    BorderStyle::default(),
                ));
                let filled = Bounds {
                    size: size(bounds.size.width * fraction, track_height),
                    ..track
                };
                if fraction > 0.0 {
                    window.paint_quad(quad(
                        filled,
                        px(3.),
                        color,
                        px(0.),
                        transparent_black(),
                        BorderStyle::default(),
                    ));
                }
                let knob_size = px(14.);
                let knob = Bounds {
                    origin: point(
                        bounds.origin.x + bounds.size.width * fraction - knob_size * 0.5,
                        bounds.origin.y + (bounds.size.height - knob_size) * 0.5,
                    ),
                    size: size(knob_size, knob_size),
                };
                window.paint_quad(quad(
                    knob,
                    px(7.),
                    color,
                    px(0.),
                    transparent_black(),
                    BorderStyle::default(),
                ));
                window.paint_quad(quad(
                    Bounds {
                        origin: point(knob.origin.x + px(4.), knob.origin.y + px(4.)),
                        size: size(px(6.), px(6.)),
                    },
                    px(3.),
                    theme::white_soft(0.75),
                    px(0.),
                    transparent_black(),
                    BorderStyle::default(),
                ));
            },
        ))
}

pub fn color_preview(color: rusted_snake_core::Rgb) -> Div {
    div()
        .size(px(30.))
        .rounded(px(8.))
        .bg(theme::from_rgb(color))
        .border_1()
        .border_color(theme::with_alpha(0xffffff, 0.18))
}

pub fn row_label(text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .child(text.into())
}

pub fn accent_bar() -> Div {
    div()
        .w(px(3.))
        .h(px(18.))
        .rounded_full()
        .bg(theme::accent())
}

pub fn centered(element: impl IntoElement) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .child(element)
        .into_any_element()
}

pub fn spacer() -> Div {
    div().flex_1()
}

pub fn mono_value(text: impl Into<SharedString>) -> Div {
    div()
        .font_family("monospace")
        .text_size(px(12.))
        .text_color(theme::text())
        .child(text.into())
}

pub fn field_id(id: &'static str) -> ElementId {
    ElementId::Name(id.into())
}

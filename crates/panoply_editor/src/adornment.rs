//! A menu button that fits inside a text input's box, beside its other adornments, such as a
//! filter box's ×: plain, an icon and a small chevron, its padding trimmed. Feathers has none; kept
//! free of Panoply's types so that it may be offered upstream.

use bevy::{
    feathers::{
        constants::icons,
        controls::{ButtonVariant, FeathersMenuToolButton},
        display::icon,
    },
    prelude::*,
};

/// The button, showing `image`; spawn it in a `FeathersMenu` beside the menu's popup.
pub fn adornment_menu_button(image: &'static str) -> impl Scene {
    bsn! {
        @FeathersMenuToolButton {
            @caption: bsn_list! {
                @icon(image)
                --
                @icon(icons::CHEVRON_DOWN) Node { height: px(8) }
            },
            @arrow: false,
        }
        ButtonVariant::Plain
        Node { padding: UiRect::horizontal(px(2)), min_width: px(0), column_gap: px(1) }
    }
}

//! A brief message at the bottom of the window, which fades by itself and takes no input. A new one
//! replaces any still showing, so a run of undos shows only the latest.

use std::borrow::Cow;

use bevy::{
    feathers::{theme::UiTheme, tokens},
    prelude::*,
};

/// Seconds a toast shows at full strength.
const TOAST_HOLD: f32 = 3.;

/// Seconds a toast takes to fade.
const TOAST_FADE: f32 = 0.5;

const TOAST_FONT_SIZE: f32 = 16.;

/// Above dialogs, and below the barrier of a save.
const TOAST_Z: i32 = 900;

/// Shows a toast.
#[derive(Message, Clone, Debug)]
pub struct ShowToast(pub Cow<'static, str>);

/// A toast's root, and how long it has shown.
#[derive(Component, Default)]
struct Toast {
    age: f32,
}

/// The toast's message, on its colored background.
#[derive(Component)]
struct ToastText;

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShowToast>()
            .add_systems(Update, (show_toast, fade_toasts).chain());
    }
}

fn show_toast(
    mut messages: MessageReader<ShowToast>,
    toasts: Query<Entity, With<Toast>>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let Some(ShowToast(message)) = messages.read().last() else {
        return;
    };
    for toast in &toasts {
        commands.entity(toast).despawn();
    }
    commands.spawn((
        Name::new("Toast"),
        Toast::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            right: Val::Px(0.),
            bottom: Val::Px(24.),
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(TOAST_Z),
        Pickable::IGNORE,
        children![(
            Name::new("ToastText"),
            ToastText,
            Text::new(message.clone()),
            TextFont {
                font: FontSource::Handle(
                    asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf")
                ),
                font_size: FontSize::Px(TOAST_FONT_SIZE),
                ..default()
            },
            TextColor(theme.color(&tokens::DIALOG_TEXT)),
            BackgroundColor(theme.color(&tokens::DIALOG_BG)),
            Node {
                padding: UiRect::axes(Val::Px(12.), Val::Px(6.)),
                border_radius: BorderRadius::all(Val::Px(4.)),
                ..default()
            },
            Pickable::IGNORE,
        )],
    ));
}

/// Ages each toast, fading its text and background, and removes it at the end of the fade.
fn fade_toasts(
    time: Res<Time<Real>>,
    mut toasts: Query<(Entity, &mut Toast, &Children)>,
    mut texts: Query<(&mut TextColor, &mut BackgroundColor), With<ToastText>>,
    mut commands: Commands,
) {
    for (entity, mut toast, children) in &mut toasts {
        toast.age += time.delta_secs();
        if toast.age >= TOAST_HOLD + TOAST_FADE {
            commands.entity(entity).despawn();
            continue;
        }
        if toast.age <= TOAST_HOLD {
            continue;
        }
        let alpha = 1. - (toast.age - TOAST_HOLD) / TOAST_FADE;
        for child in children {
            if let Ok((mut color, mut background)) = texts.get_mut(*child) {
                color.0.set_alpha(alpha);
                background.0.set_alpha(alpha);
            }
        }
    }
}

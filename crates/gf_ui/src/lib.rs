//! Loading screen and pause menu.

use bevy::prelude::*;
use gf_core::{AppState, PauseState};
use gf_input::{Back, Pause, Start, menu_input};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_menu_input)
            .add_systems(OnEnter(AppState::Loading), spawn_loading_screen)
            .add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
            .add_systems(
                Update,
                (button_feedback, quit_button).run_if(in_state(PauseState::Paused)),
            )
            .add_observer(open_pause_menu)
            .add_observer(close_pause_menu);
    }
}

const TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
const OVERLAY: Color = Color::srgba(0.02, 0.03, 0.05, 0.65);
const BUTTON_IDLE: Color = Color::srgb(0.15, 0.17, 0.22);
const BUTTON_HOVERED: Color = Color::srgb(0.25, 0.28, 0.36);
const BUTTON_PRESSED: Color = Color::srgb(0.85, 0.45, 0.12);

#[derive(Component)]
struct QuitButton;

fn spawn_menu_input(mut commands: Commands) {
    commands.spawn((Name::new("MenuInput"), menu_input()));
}

fn open_pause_menu(_: On<Start<Pause>>, mut next: ResMut<NextState<PauseState>>) {
    next.set(PauseState::Paused);
}

fn close_pause_menu(_: On<Start<Back>>, mut next: ResMut<NextState<PauseState>>) {
    next.set(PauseState::Running);
}

fn full_screen(background: Color) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(24),
            ..default()
        },
        BackgroundColor(background),
    )
}

fn label(text: &str, size: f32) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(TEXT),
    )
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        Name::new("LoadingScreen"),
        DespawnOnExit(AppState::Loading),
        full_screen(Color::BLACK),
        children![label("Loading...", 32.0)],
    ));
}

fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        Name::new("PauseMenu"),
        DespawnOnExit(PauseState::Paused),
        full_screen(OVERLAY),
        children![
            label("Paused", 56.0),
            (
                QuitButton,
                Button,
                Node {
                    width: px(220),
                    height: px(56),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BorderColor::all(TEXT),
                BackgroundColor(BUTTON_IDLE),
                children![label("Quit", 28.0)],
            ),
            label("Esc to resume", 18.0),
        ],
    ));
}

fn button_feedback(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut background) in &mut buttons {
        background.0 = match interaction {
            Interaction::Pressed => BUTTON_PRESSED,
            Interaction::Hovered => BUTTON_HOVERED,
            Interaction::None => BUTTON_IDLE,
        };
    }
}

fn quit_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        exit.write(AppExit::Success);
    }
}

//! Pause menu with tabs. Add a tab by extending [`PauseTab`] and spawning its panel.

use bevy::prelude::*;
use gf_core::PauseState;
use gf_input::{Back, Pause, Start, menu_input};
use gf_render::{AntiAliasing, GraphicsPreset, GraphicsSettings, ShadowQuality};

use crate::widgets::{
    BUTTON_ACTIVE, BUTTON_HOVERED, BUTTON_IDLE, MUTED_TEXT, OVERLAY, PANEL, button, colored_label,
    full_screen, label,
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<PauseTab>()
        .add_systems(Startup, spawn_menu_input)
        .add_systems(OnEnter(PauseState::Paused), spawn_pause_menu)
        .add_systems(
            Update,
            (
                handle_actions,
                show_active_tab,
                refresh_option_values,
                button_colors,
            )
                .chain()
                .run_if(in_state(PauseState::Paused)),
        )
        .add_observer(open_pause_menu)
        .add_observer(close_pause_menu);
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
enum PauseTab {
    #[default]
    Game,
    Graphics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsOption {
    Preset,
    Shadows,
    AntiAliasing,
    VSync,
}

impl GraphicsOption {
    fn label(self) -> &'static str {
        match self {
            Self::Preset => "Preset",
            Self::Shadows => "Shadows",
            Self::AntiAliasing => "Anti-aliasing",
            Self::VSync => "VSync",
        }
    }

    fn value(self, settings: &GraphicsSettings) -> &'static str {
        match self {
            Self::Preset => settings.preset().map_or("Custom", GraphicsPreset::label),
            Self::Shadows => settings.shadows.label(),
            Self::AntiAliasing => settings.anti_aliasing.label(),
            Self::VSync => on_off(settings.vsync),
        }
    }

    fn step(self, settings: &mut GraphicsSettings, step: i32) {
        match self {
            Self::Preset => {
                let current = settings
                    .preset()
                    .and_then(|preset| GraphicsPreset::ALL.iter().position(|p| *p == preset));
                // From a custom setup, ">" starts at the lowest preset and "<" at the highest.
                let index = current.map_or(if step > 0 { -1 } else { 0 }, |i| i as i32);
                *settings = wrap(&GraphicsPreset::ALL, index + step).settings();
            }
            Self::Shadows => settings.shadows = cycle(&ShadowQuality::ALL, settings.shadows, step),
            Self::AntiAliasing => {
                settings.anti_aliasing = cycle(&AntiAliasing::ALL, settings.anti_aliasing, step);
            }
            Self::VSync => settings.vsync = !settings.vsync,
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
enum MenuAction {
    Quit,
    OpenTab(PauseTab),
    Step(GraphicsOption, i32),
}

#[derive(Component)]
struct TabPanel(PauseTab);

#[derive(Component)]
struct OptionValue(GraphicsOption);

fn on_off(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

fn wrap<T: Copy>(all: &[T], index: i32) -> T {
    all[index.rem_euclid(all.len() as i32) as usize]
}

fn cycle<T: Copy + PartialEq>(all: &[T], current: T, step: i32) -> T {
    let index = all.iter().position(|value| *value == current).unwrap_or(0);
    wrap(all, index as i32 + step)
}

fn spawn_menu_input(mut commands: Commands) {
    commands.spawn((Name::new("MenuInput"), menu_input()));
}

fn open_pause_menu(_: On<Start<Pause>>, mut next: ResMut<NextState<PauseState>>) {
    next.set(PauseState::Paused);
}

fn close_pause_menu(_: On<Start<Back>>, mut next: ResMut<NextState<PauseState>>) {
    next.set(PauseState::Running);
}

fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        Name::new("PauseMenu"),
        DespawnOnExit(PauseState::Paused),
        full_screen(OVERLAY),
        children![
            label("Paused", 56.0),
            (
                Node {
                    width: px(460),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(20)),
                    row_gap: px(16),
                    border_radius: BorderRadius::all(px(10)),
                    ..default()
                },
                BackgroundColor(PANEL),
                children![tab_bar(), game_panel(), graphics_panel()],
            ),
            colored_label("Esc to resume", 18.0, MUTED_TEXT),
        ],
    ));
}

fn tab_bar() -> impl Bundle {
    (
        Node {
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![
            button("Game", 140.0, 40.0, MenuAction::OpenTab(PauseTab::Game)),
            button(
                "Graphics",
                140.0,
                40.0,
                MenuAction::OpenTab(PauseTab::Graphics)
            ),
        ],
    )
}

fn panel(tab: PauseTab) -> impl Bundle {
    (
        TabPanel(tab),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
            min_height: px(200),
            justify_content: JustifyContent::Center,
            ..default()
        },
    )
}

fn game_panel() -> impl Bundle {
    (
        panel(PauseTab::Game),
        children![button("Quit", 220.0, 52.0, MenuAction::Quit)],
    )
}

fn graphics_panel() -> impl Bundle {
    (
        panel(PauseTab::Graphics),
        children![
            option_row(GraphicsOption::Preset),
            option_row(GraphicsOption::Shadows),
            option_row(GraphicsOption::AntiAliasing),
            option_row(GraphicsOption::VSync),
        ],
    )
}

fn option_row(option: GraphicsOption) -> impl Bundle {
    (
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },
        children![
            (
                label(option.label(), 20.0),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ),
            button("<", 36.0, 36.0, MenuAction::Step(option, -1)),
            (
                Node {
                    width: px(120),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(
                    OptionValue(option),
                    label("", 20.0),
                    TextLayout::linebreak(LineBreak::NoWrap),
                )],
            ),
            button(">", 36.0, 36.0, MenuAction::Step(option, 1)),
        ],
    )
}

fn handle_actions(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut tab: ResMut<PauseTab>,
    mut settings: ResMut<GraphicsSettings>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
            MenuAction::OpenTab(new_tab) => *tab = new_tab,
            MenuAction::Step(option, step) => option.step(&mut settings, step),
        }
    }
}

fn show_active_tab(tab: Res<PauseTab>, mut panels: Query<(&TabPanel, &mut Node)>) {
    for (panel, mut node) in &mut panels {
        let display = if panel.0 == *tab {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

fn refresh_option_values(
    settings: Res<GraphicsSettings>,
    mut values: Query<(Ref<OptionValue>, &mut Text)>,
) {
    for (option, mut text) in &mut values {
        if settings.is_changed() || option.is_added() {
            let value = option.0.value(&settings);
            if text.0 != value {
                text.0 = value.to_string();
            }
        }
    }
}

fn button_colors(
    tab: Res<PauseTab>,
    mut buttons: Query<(&Interaction, &MenuAction, &mut BackgroundColor)>,
) {
    for (interaction, action, mut background) in &mut buttons {
        let color = match interaction {
            Interaction::Pressed | Interaction::Hovered => BUTTON_HOVERED,
            Interaction::None => match action {
                MenuAction::OpenTab(button_tab) if *button_tab == *tab => BUTTON_ACTIVE,
                _ => BUTTON_IDLE,
            },
        };
        if background.0 != color {
            background.0 = color;
        }
    }
}

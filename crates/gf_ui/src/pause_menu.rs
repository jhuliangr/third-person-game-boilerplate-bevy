//! Pause menu with tabs, usable with mouse, keyboard and gamepad.
//!
//! Add a tab by extending [`PauseTab`] and spawning its panel. Rows that can be selected
//! with the keyboard or gamepad carry a [`Focusable`] with a contiguous `order` per tab.

use bevy::prelude::*;
use gf_core::PauseState;
use gf_input::{Back, Confirm, Fire, Navigate, NextTab, Pause, PreviousTab, Start, menu_input};
use gf_render::{AntiAliasing, GraphicsPreset, GraphicsSettings, ShadowQuality};

use crate::widgets::{
    BUTTON_ACTIVE, BUTTON_HOVERED, BUTTON_IDLE, MUTED_TEXT, OVERLAY, PANEL, button, colored_label,
    full_screen, label,
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<PauseTab>()
        .init_resource::<MenuFocus>()
        .add_systems(Startup, spawn_menu_input)
        .add_systems(OnEnter(PauseState::Paused), (reset_focus, spawn_pause_menu))
        .add_systems(
            Update,
            (
                handle_clicks,
                focus_follows_mouse,
                reset_focus.run_if(resource_changed::<PauseTab>),
                show_active_tab,
                refresh_option_values,
                button_colors,
                focus_highlight,
            )
                .chain()
                .run_if(in_state(PauseState::Paused)),
        )
        .add_observer(open_pause_menu)
        .add_observer(close_pause_menu)
        .add_observer(navigate)
        .add_observer(confirm)
        .add_observer(next_tab)
        .add_observer(previous_tab);
}

const HINTS: &str =
    "Arrows / D-pad: navigate    Enter / A: select    Q E / LB RB: tabs    Esc / B: resume";

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
enum PauseTab {
    #[default]
    Game,
    Graphics,
}

impl PauseTab {
    const ALL: [Self; 2] = [Self::Game, Self::Graphics];
}

/// Position of the selected row within the active tab.
#[derive(Resource, Debug, Default)]
struct MenuFocus(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsOption {
    Preset,
    Shadows,
    AntiAliasing,
    VSync,
}

impl GraphicsOption {
    const ALL: [Self; 4] = [Self::Preset, Self::Shadows, Self::AntiAliasing, Self::VSync];

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

/// A row that keyboard and gamepad navigation can select.
#[derive(Component, Debug, Clone, Copy)]
struct Focusable {
    tab: PauseTab,
    order: usize,
    target: FocusTarget,
}

#[derive(Debug, Clone, Copy)]
enum FocusTarget {
    /// Confirm triggers the action.
    Action(MenuAction),
    /// Left/right change the value, confirm steps it forward.
    Option(GraphicsOption),
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

fn perform(
    action: MenuAction,
    tab: &mut PauseTab,
    settings: &mut GraphicsSettings,
    exit: &mut MessageWriter<AppExit>,
) {
    match action {
        MenuAction::Quit => {
            exit.write(AppExit::Success);
        }
        MenuAction::OpenTab(new_tab) => *tab = new_tab,
        MenuAction::Step(option, step) => option.step(settings, step),
    }
}

fn focused(tab: PauseTab, focus: &MenuFocus, focusables: &Query<&Focusable>) -> Option<Focusable> {
    focusables
        .iter()
        .find(|item| item.tab == tab && item.order == focus.0)
        .copied()
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

fn navigate(
    input: On<Fire<Navigate>>,
    tab: Res<PauseTab>,
    mut focus: ResMut<MenuFocus>,
    mut settings: ResMut<GraphicsSettings>,
    focusables: Query<&Focusable>,
) {
    let direction = input.value;
    if direction.y.abs() >= direction.x.abs() {
        let count = focusables.iter().filter(|item| item.tab == *tab).count();
        if count > 0 {
            let step = if direction.y > 0.0 { -1 } else { 1 };
            focus.0 = (focus.0 as i32 + step).rem_euclid(count as i32) as usize;
        }
    } else if let Some(Focusable {
        target: FocusTarget::Option(option),
        ..
    }) = focused(*tab, &focus, &focusables)
    {
        option.step(&mut settings, direction.x.signum() as i32);
    }
}

fn confirm(
    _: On<Start<Confirm>>,
    mut tab: ResMut<PauseTab>,
    focus: Res<MenuFocus>,
    mut settings: ResMut<GraphicsSettings>,
    mut exit: MessageWriter<AppExit>,
    focusables: Query<&Focusable>,
) {
    match focused(*tab, &focus, &focusables).map(|item| item.target) {
        Some(FocusTarget::Action(action)) => perform(action, &mut tab, &mut settings, &mut exit),
        Some(FocusTarget::Option(option)) => option.step(&mut settings, 1),
        None => {}
    }
}

fn next_tab(_: On<Start<NextTab>>, mut tab: ResMut<PauseTab>) {
    *tab = cycle(&PauseTab::ALL, *tab, 1);
}

fn previous_tab(_: On<Start<PreviousTab>>, mut tab: ResMut<PauseTab>) {
    *tab = cycle(&PauseTab::ALL, *tab, -1);
}

fn reset_focus(mut focus: ResMut<MenuFocus>) {
    focus.0 = 0;
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
            colored_label(HINTS, 16.0, MUTED_TEXT),
        ],
    ));
}

fn tab_bar() -> impl Bundle {
    (
        Node {
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            colored_label("LB", 16.0, MUTED_TEXT),
            button("Game", 140.0, 40.0, MenuAction::OpenTab(PauseTab::Game)),
            button(
                "Graphics",
                140.0,
                40.0,
                MenuAction::OpenTab(PauseTab::Graphics)
            ),
            colored_label("RB", 16.0, MUTED_TEXT),
        ],
    )
}

fn panel(tab: PauseTab) -> impl Bundle {
    (
        TabPanel(tab),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            min_height: px(220),
            justify_content: JustifyContent::Center,
            ..default()
        },
    )
}

fn game_panel() -> impl Bundle {
    (
        panel(PauseTab::Game),
        children![button(
            "Quit",
            220.0,
            52.0,
            (
                MenuAction::Quit,
                Focusable {
                    tab: PauseTab::Game,
                    order: 0,
                    target: FocusTarget::Action(MenuAction::Quit),
                },
            )
        )],
    )
}

fn graphics_panel() -> impl Bundle {
    let [preset, shadows, anti_aliasing, vsync] = GraphicsOption::ALL;
    (
        panel(PauseTab::Graphics),
        children![
            option_row(preset, 0),
            option_row(shadows, 1),
            option_row(anti_aliasing, 2),
            option_row(vsync, 3),
        ],
    )
}

fn option_row(option: GraphicsOption, order: usize) -> impl Bundle {
    (
        Focusable {
            tab: PauseTab::Graphics,
            order,
            target: FocusTarget::Option(option),
        },
        Interaction::default(),
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::axes(px(8), px(3)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BorderColor::all(Color::NONE),
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

fn handle_clicks(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    mut tab: ResMut<PauseTab>,
    mut settings: ResMut<GraphicsSettings>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed {
            perform(*action, &mut tab, &mut settings, &mut exit);
        }
    }
}

fn focus_follows_mouse(
    mut focus: ResMut<MenuFocus>,
    hovered: Query<(&Interaction, &Focusable), Changed<Interaction>>,
) {
    for (interaction, item) in &hovered {
        if *interaction != Interaction::None {
            focus.0 = item.order;
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
        background.set_if_neq(BackgroundColor(color));
    }
}

fn focus_highlight(
    tab: Res<PauseTab>,
    focus: Res<MenuFocus>,
    mut items: Query<(&Focusable, &mut BorderColor)>,
) {
    for (item, mut border) in &mut items {
        let color = if item.tab == *tab && item.order == focus.0 {
            BUTTON_ACTIVE
        } else {
            match item.target {
                FocusTarget::Action(_) => MUTED_TEXT,
                FocusTarget::Option(_) => Color::NONE,
            }
        };
        border.set_if_neq(BorderColor::all(color));
    }
}

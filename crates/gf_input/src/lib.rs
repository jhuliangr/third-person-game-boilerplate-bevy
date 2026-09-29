//! Input contexts, actions and their default bindings.
//!
//! Gameplay code never reads keys directly: it reacts to the actions defined here,
//! so rebinding or adding a gamepad only touches this crate.

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_enhanced_input::prelude::{Press, *};
use gf_core::PauseState;

pub use bevy_enhanced_input::prelude::{Action, Actions, Complete, Fire, Start};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .init_resource::<InputSettings>()
            .add_input_context::<Gameplay>()
            .add_input_context::<Menu>()
            .sync_context_to_state::<Gameplay, PauseState>()
            .sync_context_to_state::<Menu, PauseState>()
            .add_systems(OnEnter(PauseState::Running), grab_cursor)
            .add_systems(OnExit(PauseState::Running), release_cursor);
    }
}

/// Player preferences applied on top of the raw action values.
#[derive(Resource, Debug, Clone)]
pub struct InputSettings {
    pub look_sensitivity: f32,
    pub invert_y: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            look_sensitivity: 1.0,
            invert_y: false,
        }
    }
}

/// On-foot controls. Lives on the controlled character.
#[derive(Component)]
pub struct Gameplay;

/// Menu navigation. Lives on an entity owned by the UI.
#[derive(Component)]
pub struct Menu;

/// Movement on the ground plane, `y` is forward.
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Move;

/// Camera rotation in radians for this frame, `x` is yaw and `y` is pitch (positive looks down).
#[derive(InputAction)]
#[action_output(Vec2)]
pub struct Look;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Sprint;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Crouch;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Pause;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Back;

const MOUSE_RADIANS_PER_PIXEL: f32 = 0.0025;
const STICK_RADIANS_PER_SECOND: f32 = 3.0;

pub fn gameplay_input() -> impl Bundle {
    (
        Gameplay,
        ContextActivity::<Gameplay>::INACTIVE,
        ActiveInStates::<Gameplay, _>::single(PauseState::Running),
        actions!(Gameplay[
            (
                Action::<Move>::new(),
                DeadZone::default(),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
            (
                Action::<Look>::new(),
                Bindings::spawn((
                    Spawn((Binding::mouse_motion(), Scale::splat(MOUSE_RADIANS_PER_PIXEL))),
                    Axial::right_stick().with((
                        DeadZone::default(),
                        Negate::y(),
                        Scale::splat(STICK_RADIANS_PER_SECOND),
                        DeltaScale::default(),
                    )),
                )),
            ),
            (
                Action::<Sprint>::new(),
                bindings![KeyCode::ShiftLeft, GamepadButton::LeftThumb],
            ),
            (
                Action::<Crouch>::new(),
                bindings![KeyCode::ControlLeft, GamepadButton::East],
            ),
            (
                Action::<Pause>::new(),
                Press::default(),
                ActionSettings {
                    require_reset: true,
                    ..default()
                },
                bindings![KeyCode::Escape, GamepadButton::Start],
            ),
        ]),
    )
}

pub fn menu_input() -> impl Bundle {
    (
        Menu,
        ContextActivity::<Menu>::INACTIVE,
        ActiveInStates::<Menu, _>::single(PauseState::Paused),
        actions!(
            Menu[(
                Action::<Back>::new(),
                Press::default(),
                ActionSettings {
                    require_reset: true,
                    ..default()
                },
                bindings![KeyCode::Escape, GamepadButton::Start, GamepadButton::East],
            )]
        ),
    )
}

fn grab_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

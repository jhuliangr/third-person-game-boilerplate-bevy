//! Loading screen and pause menu.

mod loading;
mod pause_menu;
mod widgets;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((loading::plugin, pause_menu::plugin));
    }
}

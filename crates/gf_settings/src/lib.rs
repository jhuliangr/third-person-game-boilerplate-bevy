//! Persistent user settings.
//!
//! Each settings resource is stored in its own RON file inside the user's config
//! directory (`%APPDATA%\game-foundation` on Windows). Register a resource with
//! [`SettingsAppExt::register_settings`]: it is loaded when the app is built and saved
//! whenever it changes.

use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

pub struct SettingsPlugin {
    /// Folder name inside the OS config directory.
    pub app_name: &'static str,
}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let dir = dirs::config_dir().map(|dir| dir.join(self.app_name));
        if dir.is_none() {
            warn!("No config directory found; settings will not be saved");
        }
        app.insert_resource(SettingsDir(dir));
    }
}

#[derive(Resource)]
struct SettingsDir(Option<PathBuf>);

#[derive(Resource)]
struct SettingsFile<T> {
    path: Option<PathBuf>,
    _marker: PhantomData<T>,
}

pub trait SettingsAppExt {
    /// Loads `T` from `<config dir>/<name>.ron` (or uses its default) and saves it on change.
    fn register_settings<T>(&mut self, name: &str) -> &mut Self
    where
        T: Resource + Serialize + DeserializeOwned + Default;
}

impl SettingsAppExt for App {
    fn register_settings<T>(&mut self, name: &str) -> &mut Self
    where
        T: Resource + Serialize + DeserializeOwned + Default,
    {
        let path = self
            .world()
            .get_resource::<SettingsDir>()
            .expect("SettingsPlugin must be added before registering settings")
            .0
            .as_ref()
            .map(|dir| dir.join(format!("{name}.ron")));

        let settings = path.as_deref().and_then(load::<T>).unwrap_or_default();
        self.insert_resource(settings)
            .insert_resource(SettingsFile::<T> {
                path,
                _marker: PhantomData,
            })
            .add_systems(
                Last,
                save::<T>.run_if(resource_changed::<T>.and_then(not(resource_added::<T>))),
            )
    }
}

fn load<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    ron::from_str(&text)
        .inspect_err(|error| warn!("Ignoring invalid settings file {path:?}: {error}"))
        .ok()
}

fn save<T: Resource + Serialize>(settings: Res<T>, file: Res<SettingsFile<T>>) {
    let Some(path) = &file.path else {
        return;
    };
    let result = ron::ser::to_string_pretty(&*settings, ron::ser::PrettyConfig::default())
        .map_err(|error| error.to_string())
        .and_then(|text| {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|error| error.to_string())?;
            }
            fs::write(path, text).map_err(|error| error.to_string())
        });
    if let Err(error) = result {
        warn!("Could not save settings to {path:?}: {error}");
    }
}

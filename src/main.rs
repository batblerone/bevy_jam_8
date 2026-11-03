// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod asset_tracking;
mod audio;
#[cfg(feature = "dev")]
mod dev_tools;
mod game;
mod game_manager;
mod menus;
mod save;
mod screens;
mod theme;

use crate::game_manager::*;
use avian2d::prelude::*;
use bevy::{asset::AssetMetaCheck, camera::ScalingMode, prelude::*};

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy plugins.
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "bevy_jam_8".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        );

        // initialize the physics plugins
        app.add_plugins(PhysicsPlugins::default());

        // Initialize the game manager before other custom plugins
        app.add_plugins(game_manager::plugin);
        app.add_plugins(save::plugin);

        // Add other plugins.
        app.add_plugins((
            asset_tracking::plugin,
            audio::plugin,
            game::plugin,
            #[cfg(feature = "dev")]
            dev_tools::plugin,
            menus::plugin,
            screens::plugin,
            theme::plugin,
        ));

        // Order new `AppSystems` variants by adding them here:
        app.configure_sets(
            Update,
            (
                AppSystems::TickTimers,
                AppSystems::RecordInput,
                AppSystems::Update,
                AppSystems::ResolveDeaths,
            )
                .chain(),
        );

        // Set up the `Pause` state.
        app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))));

        // Spawn the main camera.
        app.add_systems(Startup, init_camera);
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
/// When adding a new variant, make sure to order it in the `configure_sets`
/// call above.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum AppSystems {
    /// Tick timers.
    TickTimers,
    /// Record player input.
    RecordInput,
    /// Do everything else (consider splitting this into further variants).
    Update,
    /// Mark and despawn anything whose health hits zero this frame
    /// Runs at end of the `Update`, never in `PostUpdate`
    /// The sprite render queus component nsers on sprites in
    /// `PostUpdate`, despawning there can race inserts with
    /// then panic with `EntityDespawned`
    ResolveDeaths,
}

// This marks our main camrea
#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

fn init_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        MainCamera,
        IsDefaultUiCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            scale: 0.01, // jscales one world unit to 100 window pixels
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

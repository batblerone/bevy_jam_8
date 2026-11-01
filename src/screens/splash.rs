//! A splash screen that plays briefly at startup.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    input::common_conditions::input_just_pressed,
    prelude::*,
};

use crate::{AppSystems, screens::Screen, theme::prelude::*};

pub(super) fn plugin(app: &mut App) {
    // Spawn splash screen.
    app.insert_resource(ClearColor(SPLASH_BACKGROUND_COLOR));
    app.add_systems(OnEnter(Screen::Splash), spawn_splash_screen);
    app.add_systems(OnExit(Screen::Splash), remove_splash_index);

    // Animate splash screen.
    app.add_systems(
        Update,
        (
            tick_fade_in_out.in_set(AppSystems::TickTimers),
            (apply_fade_in_out, advance_splash).chain().in_set(AppSystems::Update),
        )
            .run_if(in_state(Screen::Splash)),
    );

    // Exit the splash screen early if the player hits escape.
    app.add_systems(
        Update,
        enter_title_screen
            .run_if(input_just_pressed(KeyCode::Escape).and_then(in_state(Screen::Splash))),
    );
}

const SPLASH_BACKGROUND_COLOR: Color = Color::srgb(0.157, 0.157, 0.157);
const SPLASH_DURATION_SECS: f32 = 2.0;
const SPLASH_FADE_DURATION_SECS: f32 = 1.0;
const SPLASH_IMAGES: &[&str] = &["images/splash.png", "images/silly_crow_logo.png"];

/// Marks a root node for new images
#[derive(Component)]
struct SplashRoot;

/// Which entry of splash images is showing
#[derive(Resource, Default)]
struct SplashIndex(usize);

fn spawn_splash_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SplashIndex(0));
    commands
        .spawn((
            widget::ui_root("Splash Screen"),
            BackgroundColor(SPLASH_BACKGROUND_COLOR),
            DespawnOnExit(Screen::Splash),
            SplashRoot,
            children![splash_image(&asset_server, SPLASH_IMAGES[0])],
        ));
}

pub fn splash_image(asset_server: &AssetServer, path: &'static str) -> impl Bundle {
    (
        Name::new(format!("Splash image {path}")),
        Node {
            width: percent(70),
            ..default()
        },
        ImageNode::new(
            asset_server
                .load_builder()
                .with_settings(|settings: &mut ImageLoaderSettings| {
                    settings.sampler = ImageSampler::linear();
                })
                .load(path),
        ),
        ImageNodeFadeInOut {
            total_duration: SPLASH_DURATION_SECS,
            fade_duration: SPLASH_FADE_DURATION_SECS,
            // A Negative t = "Not Started Yet"; alpha() clamps to 0.
            t: 0.0,
        },
    )
}

/// Move the splash along (or leave entirely)
fn advance_splash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut index: ResMut<SplashIndex>,
    root: Single<Entity, With<SplashRoot>>,
    images: Query<(Entity, &ImageNodeFadeInOut)>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    for (entity, anim) in &images {
        if anim.t < anim.total_duration {
            continue;
        }
        commands.entity(entity).despawn();
        index.0 += 1;
        match SPLASH_IMAGES.get(index.0) {
            Some(&path) => {
                commands.entity(*root).with_child(splash_image(&asset_server, path));
            }
            None => next_screen.set(Screen::Title),
        }
    }
}

fn remove_splash_index(mut commands: Commands) {
    commands.remove_resource::<SplashIndex>();
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ImageNodeFadeInOut {
    /// Total duration in seconds.
    total_duration: f32,
    /// Fade duration in seconds.
    fade_duration: f32,
    /// Current progress in seconds, between 0 and [`Self::total_duration`].
    t: f32,
}

impl ImageNodeFadeInOut {
    fn alpha(&self) -> f32 {
        // Normalize by duration.
        let t = (self.t / self.total_duration).clamp(0.0, 1.0);
        let fade = self.fade_duration / self.total_duration;

        // Regular trapezoid-shaped graph, flat at the top with alpha = 1.0.
        ((1.0 - (2.0 * t - 1.0).abs()) / fade).min(1.0)
    }
}

fn tick_fade_in_out(time: Res<Time>, mut animation_query: Query<&mut ImageNodeFadeInOut>) {
    for mut anim in &mut animation_query {
        anim.t += time.delta_secs();
    }
}

fn apply_fade_in_out(mut animation_query: Query<(&ImageNodeFadeInOut, &mut ImageNode, &mut Node)>) {
    for (anim, mut image, mut node) in &mut animation_query {
        image.color.set_alpha(anim.alpha());

        // Show only image whos time is active
        let active = anim.t >= 0.0 && anim.t < anim.total_duration;
        let display = if active { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

fn enter_title_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}

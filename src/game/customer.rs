#![allow(unused)]
#![allow(dead_code)]
//! The customer module for spawning customers and managing behavour
//!
//! Will spawn customers with same scene methodology as player.rs

const SPAWN_INVUL: f32 = 2.0;

const MAX_CUSTOMERS: usize = 15;
const SPAWN_INTERVAL: f32 = 7.0;

use avian2d::prelude::*;
#[allow(unused)]
use bevy::{
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasTemplate},
    prelude::*,
    sprite_render::AlphaMode2d,
};

use crate::{
    AppSystems, PausableSystems,
    asset_tracking::LoadResource,
    game::{animation::PlayerAnimation, interaction::*, movement::*, physics::*, shop::YSort},
    game_manager::Screen,
    screens::gameplay::Patience,
};

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<CustomerAssets>();
    app.init_resource::<SelectedCustomer>();
    app.add_systems(
        Update,
        (
            sync_atlas_size,
            // (grant_invincibility, tick_invincibility)
            //     .chain()
            //     .in_set(AppSystems::TickTimers)
            //     .in_set(PausableSystems),
        ),
    );
}

/// The stats an attribute for each kind of player.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
#[require(YSort)]
pub struct Customer {
    pub state: CustomerState,
    pub requested_potion_from_index: Option<usize>,
    pub order_timer: Timer,
    pub wander_timer: Timer,
    pub directions: Vec2,
}

#[derive(Resource)]
pub struct CustomerSpawner(Timer);

impl Default for CustomerSpawner {
    fn default() -> Self {
        Self(Timer::from_seconds(SPAWN_INTERVAL, TimerMode::Repeating))
    }
}

fn reset_spawner(mut spawner: ResMut<CustomerSpawner>) {
    spawner.0.reset();
}

#[derive(Component, Debug, Reflect, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Component)]
pub enum CustomerState {
    #[default]
    Browsing,
    WaitingForPotion,
}

pub struct CustomerStats {
    pub patience: f32,
    pub speed: f32,
    pub scale: f32,
    pub color: Color,
    pub collider_rad: f32,
}

/// Where character sprite info is stored
pub struct CustomerSpriteSheet {
    pub path: &'static str,
    pub frame_size: UVec2,
    pub columns: u32,
    pub rows: u32,
    // gap in pixels between frames in image
    pub padding: u32,
    pub offset: UVec2, // For when lots of stuff is in their
}

// From here we define our player kinds, for now it is just one enum of a player, but if we have unlockable skins or players we can add others.
#[derive(Component, Reflect, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[reflect(Component)]
pub enum CustomerKind {
    #[default]
    Bat,
    Bunny,
    Bear,
    Mouse,
    Slime,
    Snek,
}

/// Which character the next run spawns, useful for a character select screen.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct SelectedCustomer(pub CustomerKind);

impl CustomerKind {
    /// For every kind, in the same order as the enum. Asset loading walks this list.
    pub const ALL: [CustomerKind; 6] = [
        CustomerKind::Bat,
        CustomerKind::Bunny,
        CustomerKind::Bear,
        CustomerKind::Mouse,
        CustomerKind::Slime,
        CustomerKind::Snek,
    ];

    pub fn customer_sheet(self) -> CustomerSpriteSheet {
        match self {
            CustomerKind::Snek => CustomerSpriteSheet {
                path: "images/SpriteSheet.png",
                frame_size: UVec2::new(512, 256),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::new(1280, 3820),
            },
            _ => CustomerSpriteSheet {
                path: "",
                frame_size: UVec2::ZERO,
                columns: 0,
                rows: 0,
                padding: 0,
                offset: UVec2::ZERO,
            },
        }
    }

    pub fn customer_stats(self) -> CustomerStats {
        match self {
            CustomerKind::Snek => CustomerStats {
                patience: 100.0,
                speed: 500.0,
                scale: 0.75,
                color: Color::srgb(1.0, 0.5, 0.5),
                collider_rad: 20.0,
            },
            _ => CustomerStats {
                patience: 0.0,
                speed: 0.0,
                scale: 0.0,
                color: Color::srgb(1.0, 0.5, 0.5),
                collider_rad: 0.0,
            },
        }
    }

    /// This kinds position within `ALL`, and in `CustomerAssets` lists.
    fn index(self) -> usize {
        self as usize
    }
}

/// Everything that the player is: identity, stats, movement, physics, lifetime, etc.
pub fn customer_base_scene(kind: CustomerKind) -> impl Scene {
    let stats = kind.customer_stats();
    let patience = Patience::new(stats.patience);
    let collider = Collider::circle(stats.collider_rad);
    let layers = GameLayer::player_layers();
    let despawn = DespawnOnExit(Screen::Gameplay);

    bsn! {
        Name::new("Customer")
        Customer
        kind
        patience
        RigidBody::Dynamic
        collider
        LockedAxes::ROTATION_LOCKED
        GravityScale(0.0)
        CollisionEventsEnabled
        layers
        despawn
    }
}

/// The actual player character, ready to fly into the world anew
pub fn player(kind: CustomerKind, assets: &CustomerAssets) -> impl Scene {
    let animation = PlayerAnimation::new();
    let sheet = assets.sheet(kind);
    let layout = assets.layout(kind);
    let _mesh = assets.mesh(kind);
    let _material = assets.material(kind);
    let index = animation.get_atlas_index();
    let transform = Transform::from_scale(Vec3::splat(kind.customer_stats().scale));
    bsn! {
        @customer_base_scene(kind)
        // Sprite is it's own thing in bsn!. So using the sprite function of bsn
        // we can define the sprite for our player. Where image is the sheet
        // that we have loaded in our asset loader down below.
        //
        // The texture_atlas is different. This is an option from our asset loader that
        // we want to pass off into our bsn! call. To do this we have to use the template
        // where then we have to `.into()` our layout to get the `HandleTemplate` needed for
        // `TextureAtlasTemplate`.
        Sprite {
            image: sheet,
            texture_atlas: {
                Some(TextureAtlasTemplate{
                    layout: layout.into(),
                    index: index
                })
            }
        }
        // Mesh2d(mesh)
        // MeshMaterial2d::<ColorMaterial>(material)
        transform
        animation
    }
}

/// Defining and implementing the assets for the player
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct CustomerAssets {
    /// One sprite sheet per player kind, in `CustomerKind::ALL` order. Loaded from disk,
    /// so the loading screen will wait for every sprite sheet.
    #[dependency]
    sheets: Vec<Handle<Image>>,
    /// One atlas layout per kind, in the same order. Built in code
    /// with `Assets::add`, not loaded so no #[dependency]
    layouts: Vec<Handle<TextureAtlasLayout>>,
    /// The steps used in animation, this is legacy from bevy_template_2d, but may find use elsewhere.
    #[dependency]
    pub steps: Vec<Handle<AudioSource>>,
    /// For debug and primitives construction
    pub debug_mesh: Vec<Handle<Mesh>>,
    pub debug_material: Vec<Handle<ColorMaterial>>,
}

impl CustomerAssets {
    pub fn sheet(&self, kind: CustomerKind) -> Handle<Image> {
        self.sheets[kind.index()].clone()
    }

    pub fn layout(&self, kind: CustomerKind) -> Handle<TextureAtlasLayout> {
        self.layouts[kind.index()].clone()
    }

    pub fn mesh(&self, kind: CustomerKind) -> Handle<Mesh> {
        self.debug_mesh[kind.index()].clone()
    }

    pub fn material(&self, kind: CustomerKind) -> Handle<ColorMaterial> {
        self.debug_material[kind.index()].clone()
    }
}

impl FromWorld for CustomerAssets {
    fn from_world(world: &mut World) -> Self {
        // The lists below are looked up by `kind as usize`, so `ALL` must match
        // the enums's order. this catches a mismatch in dev builds.
        for (i, kind) in CustomerKind::ALL.iter().enumerate() {
            debug_assert_eq!(
                kind.index(),
                i,
                "CustomerKind::ALL must list veriants in declaration order."
            );
        }

        // A texture atlas is a fancy way to split a single image into a grid of related images.
        // an example is found here: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
        let layouts = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            CustomerKind::ALL
                .iter()
                .map(|kind| {
                    let sheet = kind.customer_sheet();
                    layouts.add(TextureAtlasLayout::from_grid(
                        sheet.frame_size,
                        sheet.columns,
                        sheet.rows,
                        Some(UVec2::splat(sheet.padding)),
                        Some(sheet.offset),
                    ))
                })
                .collect()
        };

        let debug_mesh = {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            CustomerKind::ALL
                .iter()
                .map(|kind| meshes.add(Circle::new(kind.customer_stats().collider_rad)))
                .collect()
        };

        let debug_material = {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            CustomerKind::ALL
                .iter()
                .map(|kind| {
                    materials.add(ColorMaterial {
                        color: kind.customer_stats().color,
                        alpha_mode: AlphaMode2d::Blend,
                        ..default()
                    })
                })
                .collect()
        };

        let assets = world.resource::<AssetServer>();

        let sheets = CustomerKind::ALL
            .iter()
            .map(|kind| {
                assets
                    .load_builder()
                    .with_settings(|settings: &mut ImageLoaderSettings| {
                        // Use nearest image sampling to preserve pixel art style.
                        settings.sampler = ImageSampler::nearest();
                    })
                    .load(kind.customer_sheet().path)
            })
            .collect();

        Self {
            sheets,
            layouts,
            steps: vec![
                assets.load("audio/sound_effects/step1.ogg"),
                assets.load("audio/sound_effects/step2.ogg"),
                assets.load("audio/sound_effects/step3.ogg"),
                assets.load("audio/sound_effects/step4.ogg"),
            ],
            debug_mesh,
            debug_material,
        }
    }
}

fn sync_atlas_size(
    player_assets: Option<Res<CustomerAssets>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Some(player_assets) = player_assets else {
        return;
    };

    for kind in CustomerKind::ALL {
        let Some(image) = images.get(&player_assets.sheets[kind.index()]) else {
            continue;
        };

        let Some(mut layout) = layouts.get_mut(&player_assets.layouts[kind.index()]) else {
            continue;
        };

        // read the png dimensions
        let image_size = image.size();
        if layout.size != image_size {
            layout.size = image_size;
        }
    }
}

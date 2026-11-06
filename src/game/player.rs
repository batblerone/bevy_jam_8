//! Player-specific behavior.

/// Atlas constants
const SPRITE_HEIGHT: u32 = 256;
const SPRITE_WIDTH: u32 = 450;

const SPAWN_INVUL: f32 = 2.0;

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
    screens::gameplay::Health,
};

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<PlayerAssets>();
    app.init_resource::<SelectedPlayer>();
    app.add_systems(
        Update,
        (
            sync_atlas_size,
            (grant_invincibility, tick_invincibility)
                .chain()
                .in_set(AppSystems::TickTimers)
                .in_set(PausableSystems),
        ),
    );
}

// The marker for Player
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
#[require(YSort)]
pub struct Player;

// while present, the player ignores damage and blinks
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component)]
pub struct Invincible(pub Timer);

pub fn grant_invincibility(mut commands: Commands, new_crows: Query<Entity, Added<Player>>) {
    for crow in &new_crows {
        commands.entity(crow).insert(Invincible(Timer::from_seconds(
            SPAWN_INVUL,
            TimerMode::Once,
        )));
    }
}

pub fn tick_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut crows: Query<(Entity, &mut Invincible, &mut Sprite)>,
) {
    for (crow, mut shield, mut sprite) in &mut crows {
        shield.0.tick(time.delta());
        // Blink ten times a second
        let bright = (shield.0.elapsed_secs() * 10.0) as u32 % 2 == 0;

        sprite.color.set_alpha(if bright { 1.0 } else { 0.3 });
        if shield.0.is_finished() {
            sprite.color.set_alpha(1.0);
            commands.entity(crow).remove::<Invincible>();
        }
    }
}

/// The stats an attribute for each kind of player.
pub struct PlayerStats {
    pub health: f32,
    pub speed: f32,
    pub scale: f32,
    pub color: Color,
    pub collider_rad: f32,
}

/// Where character sprite info is stored
pub struct PlayerSpriteSheet {
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
pub enum PlayerKind {
    #[default]
    OrdinaryWitch,
}

/// Which character the next run spawns, useful for a character select screen.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct SelectedPlayer(pub PlayerKind);

impl PlayerKind {
    /// For every kind, in the same order as the enum. Asset loading walks this list.
    pub const ALL: [PlayerKind; 1] = [PlayerKind::OrdinaryWitch];

    pub fn player_sheet(self) -> PlayerSpriteSheet {
        match self {
            PlayerKind::OrdinaryWitch => PlayerSpriteSheet {
                path: "images/SpriteSheet.png",
                frame_size: UVec2::new(SPRITE_HEIGHT, SPRITE_WIDTH),
                columns: 6,
                rows: 1,
                padding: 0,
                offset: UVec2::new(0, 1300),
            },
        }
    }

    pub fn player_stats(self) -> PlayerStats {
        match self {
            PlayerKind::OrdinaryWitch => PlayerStats {
                health: 100.0,
                speed: 1000.0,
                scale: 0.75,
                color: Color::srgb(1.0, 0.5, 0.5),
                collider_rad: 20.0,
            },
        }
    }

    /// This kinds position within `ALL`, and in `PlayerAssets` lists.
    fn index(self) -> usize {
        self as usize
    }
}

/// Everything that the player is: identity, stats, movement, physics, lifetime, etc.
pub fn player_base_scene(kind: PlayerKind) -> impl Scene {
    let stats = kind.player_stats();
    let controller = MovementController {
        speed: stats.speed,
        ..default()
    };
    let health = Health::new(stats.health);
    let collider = Collider::circle(stats.collider_rad);
    let layers = GameLayer::player_layers();
    let despawn = DespawnOnExit(Screen::Gameplay);

    bsn! {
        Name::new("Player")
        Player
        kind
        health
        RigidBody::Dynamic
        collider
        LockedAxes::ROTATION_LOCKED
        GravityScale(0.0)
        CollisionEventsEnabled
        layers
        controller
        @movement_controller()
        @interaction_controller()
        despawn
    }
}

/// The actual player character, ready to fly into the world anew
pub fn player(kind: PlayerKind, assets: &PlayerAssets) -> impl Scene {
    let animation = PlayerAnimation::new();
    let sheet = assets.sheet(kind);
    let layout = assets.layout(kind);
    let _mesh = assets.mesh(kind);
    let _material = assets.material(kind);
    let index = animation.get_atlas_index();
    let transform = Transform::from_scale(Vec3::splat(kind.player_stats().scale));
    bsn! {
        @player_base_scene(kind)
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
pub struct PlayerAssets {
    /// One sprite sheet per player kind, in `PlayerKind::ALL` order. Loaded from disk,
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

impl PlayerAssets {
    pub fn sheet(&self, kind: PlayerKind) -> Handle<Image> {
        self.sheets[kind.index()].clone()
    }

    pub fn layout(&self, kind: PlayerKind) -> Handle<TextureAtlasLayout> {
        self.layouts[kind.index()].clone()
    }

    pub fn mesh(&self, kind: PlayerKind) -> Handle<Mesh> {
        self.debug_mesh[kind.index()].clone()
    }

    pub fn material(&self, kind: PlayerKind) -> Handle<ColorMaterial> {
        self.debug_material[kind.index()].clone()
    }
}

impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        // The lists below are looked up by `kind as usize`, so `ALL` must match
        // the enums's order. this catches a mismatch in dev builds.
        for (i, kind) in PlayerKind::ALL.iter().enumerate() {
            debug_assert_eq!(
                kind.index(),
                i,
                "PlayerKind::ALL must list veriants in declaration order."
            );
        }

        // A texture atlas is a fancy way to split a single image into a grid of related images.
        // an example is found here: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
        let layouts = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            PlayerKind::ALL
                .iter()
                .map(|kind| {
                    let sheet = kind.player_sheet();
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
            PlayerKind::ALL
                .iter()
                .map(|kind| meshes.add(Circle::new(kind.player_stats().collider_rad)))
                .collect()
        };

        let debug_material = {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            PlayerKind::ALL
                .iter()
                .map(|kind| {
                    materials.add(ColorMaterial {
                        color: kind.player_stats().color,
                        alpha_mode: AlphaMode2d::Blend,
                        ..default()
                    })
                })
                .collect()
        };

        let assets = world.resource::<AssetServer>();

        let sheets = PlayerKind::ALL
            .iter()
            .map(|kind| {
                assets
                    .load_builder()
                    .with_settings(|settings: &mut ImageLoaderSettings| {
                        // Use nearest image sampling to preserve pixel art style.
                        settings.sampler = ImageSampler::nearest();
                    })
                    .load(kind.player_sheet().path)
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
    player_assets: Option<Res<PlayerAssets>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Some(player_assets) = player_assets else {
        return;
    };

    for kind in PlayerKind::ALL {
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

use avian2d::prelude::*;
use bevy::{
    image::{ImageLoaderSettings, ImageSampler, TextureAtlasTemplate},
    prelude::*,
    sprite_render::AlphaMode2d,
};

use crate::{
    asset_tracking::LoadResource, game::{animation::GrowBedAnimation, interaction::Interactable, physics::*, shop::YSort, stations::{cauldron::{Cauldron}, grow_bed::GrowBed}}, game_manager::Screen,
};

pub mod cauldron;
pub mod grow_bed;

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<StationAssets>();
    app.add_plugins(cauldron::plugin);
    app.add_plugins(grow_bed::plugin);
    app.register_type::<StationKind>()
        .register_type::<Station>();
    app.add_systems(Update, sync_atlas_size);
}

/// Every kind of station the witch can use.
///
/// Also used by `Station:<Kind>` point object names in Tiled.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component, Default)]
pub enum StationKind {
    #[default]
    Counter,
    Crate,
    FlaskShelf,
    Mortar,
    DryingRack,
    Cauldron,
    Bed,
    Trash,
    GrowBed,
    ServiceCounter,
}

/// A live, interactable station. The kind enum alone describes the type,
/// while this component is queried by existing item placement systems.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component, Default)]
pub struct Station {
    pub kind: StationKind,
}

/// Per-kind gameplay footprint and sprite scale. Most station art is still
/// placeholder, but the Cauldron sprite uses a region of SpriteSheet.png.
pub struct StationSpec {
    /// Entity name shown in the inspector.
    pub name: &'static str,
    /// Short text drawn on the placeholder block so you can tell them apart.
    pub label: &'static str,
    pub color: Color,
    /// Footprint in tiles. Also the collider size. Counters fill their tile
    /// completely so a row of them forms a solid, seamless barrier.
    pub size: UVec2,
    pub scale: f32,
    pub collider: Collider
}

/// The sprite sheet for a station. The loader uses this to build a `TextureAtlas`.
pub struct StationSpriteSheet {
    pub frame_size: UVec2,
    pub columns: u32,
    pub rows: u32,
    pub padding: u32,
    pub offset: UVec2,
}

impl StationKind {
    /// Every kind, in declaration order. Asset loaders walk this list.
    #[allow(dead_code)] // For the station asset loader once art arrives.
    pub const ALL: [StationKind; 10] = [
        StationKind::Counter,
        StationKind::Crate,
        StationKind::FlaskShelf,
        StationKind::Mortar,
        StationKind::DryingRack,
        StationKind::Cauldron,
        StationKind::Bed,
        StationKind::Trash,
        StationKind::GrowBed,
        StationKind::ServiceCounter,
    ];

    /// Tiled point object names use `Station:<variant>`.
    pub fn from_tiled_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| format!("{kind:?}") == name)
    }

    /// This kinds position within `ALL`, and in `PlayerAssets` lists.
    fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> StationSpec {
        let (name, label, color, size, scale, collider) = match self {
            StationKind::Counter => (
                "Counter",
                "",
                srgb(0.55, 0.36, 0.22),
                UVec2::ONE,
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::ServiceCounter => (
                "Service Counter",
                "Serve",
                srgb(0.85, 0.65, 0.20),
                UVec2::new(128, 128),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::Crate => (
                "Ingredient Crate",
                "Crate",
                srgb(0.70, 0.50, 0.30),
                UVec2::new(128, 128),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::FlaskShelf => (
                "Flask Shelf",
                "Flask",
                srgb(0.45, 0.75, 0.85),
                UVec2::new(256, 512),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::Mortar => (
                "Mortar",
                "Mortar",
                srgb(0.60, 0.60, 0.65),
                UVec2::new(512, 512),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::DryingRack => (
                "Drying Rack",
                "Dry",
                srgb(0.65, 0.55, 0.30),
                UVec2::new(512, 512),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::Cauldron => (
                "Cauldron",
                "Brew",
                srgb(0.25, 0.25, 0.30),
                UVec2::new(512, 512),
                0.8,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::Bed => (
                "Bed",
                "Bed",
                srgb(0.55, 0.35, 0.65),
                UVec2::new(256, 512),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::Trash => (
                "Trash",
                "Trash",
                srgb(0.35, 0.40, 0.35),
                UVec2::new(128, 256),
                1.0,
                Collider::rectangle(0.75, 0.75)
            ),
            StationKind::GrowBed => (
                "Grow Bed",
                "Grow",
                srgb(0.30, 0.55, 0.25),
                UVec2::new(256, 256),
                1.0,
                Collider::rectangle(1.0, 1.0)
            ),
        };
        StationSpec {
            name,
            label,
            color,
            size,
            scale,
            collider
        }
    }

    pub fn sprite_sheet(self) -> StationSpriteSheet {
        match self {
            StationKind::Cauldron => StationSpriteSheet {
                frame_size: UVec2::splat(512),
                columns: 4,
                rows: 3,
                padding: 0,
                offset: UVec2::new(1536, 2688),
            },
            StationKind::GrowBed => StationSpriteSheet {
                frame_size: UVec2::splat(256),
                columns: 6,
                rows: 1,
                padding: 0,
                offset: UVec2::new(0, 3072),
            },
            _ => StationSpriteSheet {
                frame_size: UVec2::splat(32),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::ZERO,
            },
        }
    }
}

const fn srgb(r: f32, g: f32, b: f32) -> Color {
    Color::srgb(r, g, b)
}

/// Core station behavior. It is not a player input context: interactions
/// target this entity through `Interactable` and its Avian collider.
pub fn stations_base_scene(kind: StationKind) -> impl Scene {
    let spec = kind.spec();
    let size = spec.size.x as f32 * spec.scale;
    let collider = Collider::rectangle(size, size);
    let layers = GameLayer::station_layers();
    let station = Station{ kind };
    let despawn = DespawnOnExit(Screen::Gameplay);
    // let animation = match kind{
    //     StationKind::Cauldron => CauldronAnimation::new(),
    //     StationKind::GrowBed => GrowBedAnimation::new(),
    //     _ => None
    // }

    bsn! {
        Name::new(spec.name)
        Station { kind }
        station
        Interactable
        YSort
        RigidBody::Static
        collider
        layers
        despawn
    }
}

/// Construct the station using its own loaded assets and behavior scene.
/// `shop.rs` positions the returned scene root at a Tiled point marker.
pub fn stations(kind: StationKind, assets: &StationAssets) -> impl Scene {
    let sheet = assets.sheet(kind);
    let layout = assets.layout(kind);
    let transform = Transform::from_scale(Vec3::splat(kind.spec().scale));

    bsn! {
        @stations_base_scene(kind)
        Sprite {
            image: sheet,
            texture_atlas: {
                Some(TextureAtlasTemplate {
                    layout: layout.into(),
                    index: 0
                })
            }
        }
        transform
    }
}

/// Cached texture handles and atlas layouts for station scenes.
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct StationAssets {
    /// One handle per station kind, in `StationKind::ALL` order. Currently all
    /// kinds share the same source sheet but use different atlas regions.
    #[dependency]
    sheets: Vec<Handle<Image>>,
    /// One atlas layout per kind, in the same order. Built in code
    /// with `Assets::add`, not loaded so no #[dependency]
    layouts: Vec<Handle<TextureAtlasLayout>>,
    /// For debug and primitives construction
    pub debug_mesh: Vec<Handle<Mesh>>,
    pub debug_material: Vec<Handle<ColorMaterial>>,
}

impl StationAssets {
    pub fn sheet(&self, kind: StationKind) -> Handle<Image> {
        self.sheets[kind.index()].clone()
    }

    pub fn layout(&self, kind: StationKind) -> Handle<TextureAtlasLayout> {
        self.layouts[kind.index()].clone()
    }

    pub fn mesh(&self, kind: StationKind) -> Handle<Mesh> {
        self.debug_mesh[kind.index()].clone()
    }

    pub fn material(&self, kind: StationKind) -> Handle<ColorMaterial> {
        self.debug_material[kind.index()].clone()
    }
}

impl FromWorld for StationAssets {
    fn from_world(world: &mut World) -> Self {
        // The lists below are looked up by `kind as usize`, so `ALL` must match
        // the enums's order. this catches a mismatch in dev builds.
        for (i, kind) in StationKind::ALL.iter().enumerate() {
            debug_assert_eq!(
                kind.index(),
                i,
                "StationKind::ALL must list variants in declaration order."
            );
        }

        // A texture atlas is a fancy way to split a single image into a grid of related images.
        // an example is found here: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
        let layouts = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            StationKind::ALL
                .iter()
                .map(|kind| {
                    let sheet = kind.sprite_sheet();
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
            StationKind::ALL
                .iter()
                .map(|kind| meshes.add(Circle::new(kind.spec().size.x as f32)))
                .collect()
        };

        let debug_material = {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            StationKind::ALL
                .iter()
                .map(|kind| {
                    materials.add(ColorMaterial {
                        color: kind.spec().color,
                        alpha_mode: AlphaMode2d::Blend,
                        ..default()
                    })
                })
                .collect()
        };

        let assets = world.resource::<AssetServer>();

        let sheet = assets
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load("images/SpriteSheet.png");

        let sheets: Vec<Handle<Image>> = StationKind::ALL.iter().map(|_| sheet.clone()).collect();

        Self {
            sheets,
            layouts,
            debug_mesh,
            debug_material,
        }
    }
}

fn sync_atlas_size(
    station_assets: Option<Res<StationAssets>>,
    images: Res<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Some(station_assets) = station_assets else {
        return;
    };

    for kind in StationKind::ALL {
        let Some(image) = images.get(&station_assets.sheets[kind.index()]) else {
            continue;
        };

        let Some(mut layout) = layouts.get_mut(&station_assets.layouts[kind.index()]) else {
            continue;
        };

        // read the png dimensions
        let image_size = image.size();
        if layout.size != image_size {
            layout.size = image_size;
        }
    }
}

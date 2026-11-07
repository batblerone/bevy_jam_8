use avian2d::{collision::collider::Collider, dynamics::rigid_body::RigidBody};
use bevy::{image::{ImageLoaderSettings, ImageSampler, TextureAtlasTemplate}, prelude::*, sprite_render::AlphaMode2d};
use std::path::PathBuf;

use crate::{game::{interaction::Interactable, physics::GameLayer, shop::YSort}, game_manager::Screen};

// ==========================================
// UNIQUE SLOT INGREDIENT EFFECT POOLS
// ==========================================

/// Every ingredient has its own unique pool of slot outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseEffect {
    Fire=0,
    Light=1,
    StickyWeb=2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierEffect {
    Explosion=0,
    Resistance=1,
    Blindness=2,
}
    
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtilityEffect {
    Replicate=0,
    Smoothen=1,
    Electrify=2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeliveryEffect {
    Splash=0,
    Stretch=1,
    Aura=2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngredientEffect {
    Base(BaseEffect),
    Modifier(ModifierEffect),
    Utility(UtilityEffect),
    Delivery(DeliveryEffect),
}

// ==========================================
// INGREDIENT & RECIPE STRUCTURES
// ==========================================
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component, Default)]
pub enum IngredientKind {
    #[default]
    Fireweed=0,
    Silkworm=1,
    Glowcap=2,
}

impl IngredientKind {
    /// Every kind, in declaration order. Asset loaders walk this list.
    // For the station asset loader once art arrives.
    pub const ALL: [IngredientKind; 3] = [
        IngredientKind::Fireweed,
        IngredientKind::Silkworm,
        IngredientKind::Glowcap,
    ];

    fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> IngredientSpec {
        let (name, label, color, size, scale) = match self {
            IngredientKind::Fireweed => (
                "Fireweed",
                "Harvest",
                Color::srgb(0.85, 0.65, 0.20),
                Vec2::ONE,
                1.0,
            ),
            _ => (
                "",
                "",
                Color::srgb(0.0, 0.0, 0.0),
                Vec2::ONE,
                1.0,
            )
        };
        IngredientSpec {
            name,
            label,
            color,
            size,
            scale,
        }
    }

    pub fn sprite_sheet(self) -> IngredientSpriteSheet {
        match self {
            IngredientKind::Fireweed => IngredientSpriteSheet {
                frame_size: UVec2::splat(128),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::new(1536, 2688),
            },
            IngredientKind::Silkworm => IngredientSpriteSheet {
                frame_size: UVec2::splat(128),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::new(0, 3072),
            },
            IngredientKind::Glowcap => IngredientSpriteSheet {
                frame_size: UVec2::splat(128),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::new(0, 3072),
            },
            _ => IngredientSpriteSheet {
                frame_size: UVec2::splat(32),
                columns: 1,
                rows: 1,
                padding: 0,
                offset: UVec2::ZERO,
            },
        }
    }
}

/// Per-kind gameplay footprint and sprite scale.
pub struct IngredientSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub color: Color,
    pub size: Vec2,
    pub scale: f32,
}

/// The sprite sheet for a station. The loader uses this to build a `TextureAtlas`.
pub struct IngredientSpriteSheet {
    pub frame_size: UVec2,
    pub columns: u32,
    pub rows: u32,
    pub padding: u32,
    pub offset: UVec2,
}

/// Contains data regarding a single ingredient asset.
#[derive(Component, Debug, Clone)]
pub struct Ingredient {
    pub kind: IngredientKind,
}

impl Ingredient {
    /// Extracts the specific effect based strictly on its positional sequence index (0 to 3).
    pub fn get_effect_by_slot_index(&self, index: usize) -> Option<IngredientEffect> {
        match index {
        0 => unsafe {
            Some(IngredientEffect::Base(std::mem::transmute(self.kind as u8)))
        },
        1 => unsafe {
            Some(IngredientEffect::Modifier(std::mem::transmute(self.kind as u8)))
        },
        2 => unsafe {
            Some(IngredientEffect::Utility(std::mem::transmute(self.kind as u8)))
        },
        3 => unsafe {
            Some(IngredientEffect::Delivery(std::mem::transmute(self.kind as u8)))
        },
        _ => None,
        }
    }
}

/// Recipe configuration supporting anywhere from 1 to 4 generic sequential ingredients.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub ingredients: Vec<IngredientKind>,
}

/// A specific potion containing the unique compiled list of active ingredient effects.
#[derive(Debug, Clone)]
pub struct Potion {
    pub name: String,
    pub active_effects: Vec<IngredientEffect>,
    pub recipe : Recipe,
}

// ==========================================
// DATABASE AND CRAFTING LOGIC
// ==========================================
#[derive(Resource, Debug, Clone, Default)]
pub struct AlchemyDatabase {
    pub ingredients: Vec<Ingredient>,
    pub potions: Vec<Potion>,
}

impl AlchemyDatabase {
    pub fn new() -> Self {
        Self {
            ingredients: vec![
                Ingredient {
                    kind: IngredientKind::Fireweed
                },
                Ingredient {
                    kind: IngredientKind::Silkworm
                },
                Ingredient {
                    kind: IngredientKind::Glowcap
                },
            ],
            potions: vec![
                Potion {
                    name: "Potion of Fire".to_string(),
                    active_effects: vec![IngredientEffect::Base(BaseEffect::Fire)],
                    recipe: Recipe {
                        ingredients: vec![IngredientKind::Fireweed],
                    },
                },
                Potion {
                    name: "Potion of Fire Resistance".to_string(),
                    active_effects: vec![
                        IngredientEffect::Base(BaseEffect::Fire),
                        IngredientEffect::Modifier(ModifierEffect::Resistance),
                    ],
                    recipe: Recipe {
                        ingredients: vec![IngredientKind::Fireweed, IngredientKind::Silkworm],
                    },
                },
                Potion {
                    name: "Flash of Light".to_string(),
                    active_effects: vec![
                        IngredientEffect::Base(BaseEffect::Light),
                        IngredientEffect::Modifier(ModifierEffect::Explosion),
                    ],
                    recipe: Recipe {
                        ingredients: vec![IngredientKind::Glowcap, IngredientKind::Fireweed],
                    },
                },
            ],
        }
    }

    pub fn find_ingredient(&self, id: IngredientKind) -> Option<&Ingredient> {
        self.ingredients.iter().find(|i| i.kind == id)
    }

    pub fn find_potion_by_recipe(&self, recipe: &Recipe) -> Option<&Potion> {
        self.potions.iter().find(|p| p.recipe.ingredients == recipe.ingredients)
    }

    pub fn find_potion_by_ingredients(&self, ingredients: &[IngredientKind]) -> Option<&Potion> {
        self.potions.iter().find(|p| p.recipe.ingredients == ingredients)
    }

    pub fn find_potion_by_active_effects(&self, active_effects: &[IngredientEffect]) -> Option<&Potion> {
        self.potions.iter().find(|p| p.active_effects == *active_effects)
    }
}

/// Core ingredient behavior. It is not a player input context: interactions
/// target this entity through `Interactable` and its Avian collider.
pub fn ingredients_base_scene(kind: IngredientKind) -> impl Scene {
    let spec = kind.spec();
    let size = spec.size * spec.scale;
    let collider = Collider::rectangle(size.x * 0.75, size.y * 0.75);
    let layers = GameLayer::pickup_layers();
    let station = Ingredient { kind };
    let despawn = DespawnOnExit(Screen::Gameplay);

    bsn! {
        Name::new(spec.name)
        kind
        station
        Interactable
        YSort
        RigidBody::Static
        collider
        layers
        despawn
    }
}

pub fn ingredients(kind: IngredientKind, assets: &IngredientAssets) -> impl Scene {
    let sheet = assets.sheet(kind);
    let layout = assets.layout(kind);
    let transform = Transform::from_scale(Vec3::splat(kind.spec().scale));

    bsn! {
        @ingredients_base_scene(kind)
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

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct IngredientAssets {
    /// One handle per ingredient kind, in `IngredientKind::ALL` order. Currently all
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

impl IngredientAssets {
    pub fn sheet(&self, kind: IngredientKind) -> Handle<Image> {
        self.sheets[kind.index()].clone()
    }

    pub fn layout(&self, kind: IngredientKind) -> Handle<TextureAtlasLayout> {
        self.layouts[kind.index()].clone()
    }

    pub fn mesh(&self, kind: IngredientKind) -> Handle<Mesh> {
        self.debug_mesh[kind.index()].clone()
    }

    pub fn material(&self, kind: IngredientKind) -> Handle<ColorMaterial> {
        self.debug_material[kind.index()].clone()
    }
}

impl FromWorld for IngredientAssets {
    fn from_world(world: &mut World) -> Self {
        // The lists below are looked up by `kind as usize`, so `ALL` must match
        // the enums's order. this catches a mismatch in dev builds.
        for (i, kind) in IngredientKind::ALL.iter().enumerate() {
            debug_assert_eq!(
                kind.index(),
                i,
                "IngredientKind::ALL must list variants in declaration order."
            );
        }

        // A texture atlas is a fancy way to split a single image into a grid of related images.
        // an example is found here: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
        let layouts = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            IngredientKind::ALL
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
            IngredientKind::ALL
                .iter()
                .map(|kind| meshes.add(Circle::new(kind.spec().size.x)))
                .collect()
        };

        let debug_material = {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            IngredientKind::ALL
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

        let sheets: Vec<Handle<Image>> = IngredientKind::ALL.iter().map(|_| sheet.clone()).collect();

        Self {
            sheets,
            layouts,
            debug_mesh,
            debug_material,
        }
    }
}
use bevy::{prelude::*};
use std::path::PathBuf;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngredientKind {
    Fireweed=0,
    Silkworm=1,
    Glowcap=2,
}

/// Contains data regarding a single ingredient asset.
#[derive(Debug, Clone)]
pub struct Ingredient {
    pub id: IngredientKind,
    pub name: &'static str,
    pub icon_path: PathBuf,
}

impl Ingredient {
    /// Extracts the specific effect based strictly on its positional sequence index (0 to 3).
    pub fn get_effect_by_slot_index(&self, index: usize) -> Option<IngredientEffect> {
        match index {
        0 => unsafe {
            Some(IngredientEffect::Base(std::mem::transmute(self.id as u8)))
        },
        1 => unsafe {
            Some(IngredientEffect::Modifier(std::mem::transmute(self.id as u8)))
        },
        2 => unsafe {
            Some(IngredientEffect::Utility(std::mem::transmute(self.id as u8)))
        },
        3 => unsafe {
            Some(IngredientEffect::Delivery(std::mem::transmute(self.id as u8)))
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
    pub icon_path: PathBuf,
    pub active_effects: Vec<IngredientEffect>,
    pub recipe : Recipe,
}

// ==========================================
// 3. DATABASE AND CRAFTING LOGIC
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
                    id: IngredientKind::Fireweed,
                    name: "Fireweed",
                    icon_path: PathBuf::from("assets/icons/ingredients/fireweed.png"),
                },
                Ingredient {
                    id: IngredientKind::Silkworm,
                    name: "Silkworm",
                    icon_path: PathBuf::from("assets/icons/ingredients/silkworm.png"),
                },
                Ingredient {
                    id: IngredientKind::Glowcap,
                    name: "Glowcap",
                    icon_path: PathBuf::from("assets/icons/ingredients/glowcap.png"),
                },
            ],
            potions: vec![
                Potion {
                    name: "Potion of Fire".to_string(),
                    icon_path: PathBuf::from("assets/icons/potions/fire.png"),
                    active_effects: vec![IngredientEffect::Base(BaseEffect::Fire)],
                    recipe: Recipe {
                        ingredients: vec![IngredientKind::Fireweed],
                    },
                },
                Potion {
                    name: "Potion of Fire Resistance".to_string(),
                    icon_path: PathBuf::from("assets/icons/potions/fire_resistance.png"),
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
                    icon_path: PathBuf::from("assets/icons/potions/flash_of_light.png"),
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
        self.ingredients.iter().find(|i| i.id == id)
    }

    pub fn find_potion_by_recipe(&self, recipe: &Recipe) -> Option<&Potion> {
        self.potions.iter().find(|p| p.recipe.ingredients == recipe.ingredients)
    }

    pub fn find_potion_by_active_effects(&self, active_effects: &[IngredientEffect]) -> Option<&Potion> {
        self.potions.iter().find(|p| p.active_effects == *active_effects)
    }
}
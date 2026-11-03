//! The progression module tracks how well the player is doing throughout the game.
//! 
//! A score track will be present for the 8 days a player looks after the shop.
//! What this actually means, I have no idea.

use bevy::prelude::*;

/// Points needed before the first boss shows up
const FIRST_BOSS_AT: u32 = 500;
/// After a boss is beaten, the next one needs this many more points
/// TODO: add more progression later
const POINTS_BETWEEN_BOSSES: u32 = 750;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LevelProgress>();
}

/// Where the player is in the level progression
#[derive(Resource, Debug)]
pub struct LevelProgress {
    /// Starts at 1, goes up each time a boss is beaten
    pub level: u32,
    /// jthe score that summons this level's boss
    pub boss_threshold: u32,
    /// True from the moment the threshold is hit until the boss is beaten.
    /// Stops the same boss being summoned again while points keep coming in.
    pub boss_active: bool,
}

impl Default for LevelProgress {
    fn default() -> Self {
        Self {
            level: 1,
            boss_threshold: FIRST_BOSS_AT,
            boss_active: false,
        }
    }
}

impl LevelProgress {
    /// Call this when the boss dies. The next threshold is counted from the
    /// score at the moment it is called. A big boss reward can't instantly
    /// summon the next boss.
    #[allow(unused)]
    pub fn advance(&mut self, current_score: u32) {
        self.level += 1;
        self.boss_threshold = current_score.saturating_add(POINTS_BETWEEN_BOSSES);
        self.boss_active = false;
    }

    /// Constructor for continued runs
    pub fn starting_at(level: u32, score: u32) -> Self {
        Self {
            level,
            boss_threshold: if level <= 1 {
                FIRST_BOSS_AT
            } else {
                score.saturating_add(POINTS_BETWEEN_BOSSES)
            },
            boss_active: false,
        }
    }

    /// 0.0 on level 1, rising 0.15 per level, capped at 1.0 (level 7+).
    #[allow(unused)]
    pub fn difficulty(&self) -> f32 {
        (self.level.saturating_sub(1) as f32 * 0.15).min(1.0)
    }
}


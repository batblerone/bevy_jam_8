//! The score module tracks how well the player is doing.
//!
//! Score is tracked by successfully delivering potions to customers.
//! This will probably be some sort of [`PotionDelivered`] message
//! sent out by a potion tracking system. This remains to be seen.

use bevy::prelude::*;

use crate::game_manager::Screen;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Score>();
    app.add_message::<EnemyDefeated>();

    // The run's score is reset by `start_run` (game_manager.rs), and drawn by hud.rs.
    app.add_systems(
        Update,
        add_points_for_kills.run_if(in_state(Screen::Gameplay)),
    );
}

/// How many points the player earns for defeating an entity.
///
/// It lives within the entity (instead of being looked up from `EnemyKind`)
/// so that anything can be worth points: Such as tougher variants, or bosses,
/// without score code knowing about them.
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct PointValue(pub u32);

/// Sent once for every enemy the player defeats.
#[derive(Message, Debug, Clone, Copy)]
pub struct EnemyDefeated {
    /// Points that each kill is worth
    pub points: u32,
    /// Where the enemy died. For use later for pop-ups, or effects (+25 pts).
    #[allow(dead_code)] // Not in use yet
    pub position: Vec2,
}

/// The players actual score.
#[derive(Resource, Debug, Default)]
pub struct Score {
    /// Points earned this run
    pub current: u32,
    /// Highest score reached since game start
    pub best_score: u32,
}

impl Score {
    pub fn add(&mut self, points: u32) {
        // saturating_add can'd overflow and panic, however logn a run lasts.
        self.current = self.current.saturating_add(points);
        self.best_score = self.best_score.max(self.current);
    }
}

/// Turns kill messages into points
pub fn add_points_for_kills(mut defeated: MessageReader<EnemyDefeated>, mut score: ResMut<Score>) {
    for kill in defeated.read() {
        score.add(kill.points);
    }
}

pub fn score_label(points: u32) -> String {
    format!("Score: {points}")
}

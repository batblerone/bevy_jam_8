//! Game manager controls the game state, score tracking,
//! save states and overall progress of the game.

use bevy::{prelude::*, ui::debug};
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::game::{player::*, progression::*, score::*};

pub const STARTING_LIVES: u32 = 3;

pub(super) fn plugin(app: &mut App) {
    // All states live within this plugin, it manages game flow
    app.init_state::<Screen>();
    app.init_state::<Menu>();
    app.init_state::<Pause>();

    // GamePhase is a sub-state of Screen::Gameplay, so it only exists when the game is in that state.
    app.add_sub_state::<GamePhase>(); 

    app.init_resource::<RunCycle>();
    app.init_resource::<NightTicker>();
    app.init_resource::<Lives>();
    app.init_resource::<RunStart>();
    app.init_resource::<RunSeed>();

    app.add_message::<PlayerDied>();
    app.add_message::<NightTickEvent>();

    // OnEnter systems
    app.add_systems(OnEnter(Screen::Gameplay), start_run.in_set(RunSetup));
    app.add_systems(OnEnter(GamePhase::Day), start_day_phase);
    app.add_systems(OnEnter(GamePhase::Night), start_night_phase);

    // Update systems
    app.add_systems(Update, tick_night_clock.run_if(in_state(GamePhase::Night)));
    app.add_systems(Update, test_advance_phase.run_if(resource_exists::<State<GamePhase>>));
}

// ------ State Data for managing game flow ------

/// What is happening inside of a run. Only exists during `Screen::Gameplay`.
#[derive(SubStates, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[source(Screen = Screen::Gameplay)]
pub enum GamePhase {
    #[default]
    Initial,
    Night,
    Dawn,
    Day,
    Dusk,
    Victory,
    GameOver
}

/// Systems that will set up a new run. Anything that spawns a level runs after it.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RunSetup;

/// Menu sub states for spawning menu overlays.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Menu {
    #[default]
    None,
    Main,
    Credits,
    Settings,
    Pause,
}

/// The game's main screen states. Dictates what is played on screen for the player and controls game loop.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    Gameplay,
}

/// Whether or not the game is paused.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Pause(pub bool);

/// A system set for systems that shouldn't run while the game is paused.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PausableSystems;

// ####### Run Data for managing entities within a run ######

/// The lives rmaining, counting the one in play.
#[derive(Resource, Debug, Clone, Copy)]
pub struct Lives(pub u32);

impl Default for Lives {
    fn default() -> Self {
        Self(STARTING_LIVES)
    }
}

/// The seed for the runs level
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RunSeed(pub u32);

/// Where a run picks up from. Saved after a level clear
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub level: u32,
    pub score: u32,
    pub lives: u32,
    pub seed: u32,
}

/// Set by main menu before a run enters `Screen::Gameplay`
#[derive(Resource, Debug, Clone, Default)]
pub enum RunStart {
    #[default]
    NewGame,
    Continue(Checkpoint),
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerDied {
    #[allow(dead_code)] // for particle effects later
    pub position: Vec2,
}

#[derive(Resource, Default, Debug)]
pub struct RunCycle {
    pub current_day: u32,
}

#[derive(Resource, Debug, Clone)]
struct NightTicker {
    timer: Timer,
}
impl Default for NightTicker {
    fn default() -> Self {
        Self {
            // This is where we inject the 300 seconds (5 minutes)!
            timer: Timer::new(Duration::from_secs(300), TimerMode::Once),
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct NightTickEvent {
    /// Time elapsed in the current frame
    pub delta_time: Duration,
    /// Total time elapsed since the night started (useful for milestones/scheduling events)
    pub total_elapsed: Duration,
}

// ------ The actual systems of the game manager ------

/// One place that resets everthing a run needs.
fn start_run(
    mut run_start: ResMut<RunStart>,
    mut lives: ResMut<Lives>,
    mut score: ResMut<Score>,
    mut progress: ResMut<LevelProgress>,
    mut seed: ResMut<RunSeed>,
    mut run_cycle: ResMut<RunCycle>,
    mut next_phase: ResMut<NextState<GamePhase>>,
) {
    println!("\n\n ----- [RUN INITIALIZATION] ----- \n\n");

    // `take` resets `RunStart` to `NewGame`, so Retry after a Continue starts fresh.
    match std::mem::take(&mut *run_start) {
        RunStart::NewGame => {
            *lives = Lives::default();
            score.current = 0;
            *progress = LevelProgress::default();
            seed.0 = fastrand::u32(..);
        }
        RunStart::Continue(cp) => {
            lives.0 = cp.lives.max(1);
            score.current = cp.score;
            *progress = LevelProgress::starting_at(cp.level, cp.score);
            seed.0 = cp.seed;
        }
    }


    // Force the game phase machine to switch directly to Day
    next_phase.set(GamePhase::Day);
    // Reset the run cycle counter to Day 0
    run_cycle.current_day = 0;
}

fn tick_night_clock(
    time: Res<Time>,
    mut night_ticker: ResMut<NightTicker>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut messages: MessageWriter<NightTickEvent>, 
) {
    // Tick the timer with frame delta time
    let delta = time.delta();
    night_ticker.timer.tick(delta);

    messages.write(NightTickEvent {
        delta_time: delta,
        total_elapsed: night_ticker.timer.elapsed(),
    });

    if night_ticker.timer.just_finished() {
        println!("Moon go sleep now! To dawn with thee...");
        
        // Switch the game phase to Dawn
        next_state.set(GamePhase::Dawn);
    }
}

fn start_day_phase(mut run_cycle: ResMut<RunCycle>) {
    println!("\n ----- Starting day phase! ----- \n");

    // Increment the day counter for the run cycle
    run_cycle.current_day += 1;
    println!("Cycle {}", run_cycle.current_day);
}

/// Resets the night clock timer to 0.0 seconds
fn start_night_phase(mut night_ticker: ResMut<NightTicker>) {
    println!("\n\n ----- Starting night phase! -----\n\n");
    night_ticker.timer.reset(); 
}

/// A debug system to manually push the game from Day to Night for testing purposes
fn test_advance_phase(
    keyboard_input: Res<ButtonInput<KeyCode>>, // Reads keyboard state
    current_state: Res<State<GamePhase>>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let shift_held = keyboard_input.pressed(KeyCode::ShiftLeft) 
                  || keyboard_input.pressed(KeyCode::ShiftRight);

    let f1_pressed = keyboard_input.just_pressed(KeyCode::F1);

    if shift_held && f1_pressed {
        // Determine the next state based on the current state
        let target_phase = match current_state.get() {
            GamePhase::Day => Some(GamePhase::Dusk),
            GamePhase::Dusk => Some(GamePhase::Night),
            GamePhase::Night => Some(GamePhase::Dawn),
            GamePhase::Dawn => Some(GamePhase::Day),
            _ => None, // Do nothing for Victory or GameOver
        };

        // If a valid transition exists, execute it and print dynamically
        if let Some(next) = target_phase {
            next_state.set(next);
            println!("[DEBUG] Advancing phase: {:?} -> {:?}", current_state.get(), next);
        }
    }
}
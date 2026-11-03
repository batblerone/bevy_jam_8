use bevy::prelude::*;

use crate::game_manager::{GamePhase, NightTickEvent, Screen}; 

#[derive(Component)]
pub struct VisualTimelineTicker;

pub struct TimelineUiPlugin;

impl Plugin for TimelineUiPlugin {
    fn build(&self, app: &mut App) {
        // Uses Screen state if setup, or assign to whichever initial setup schedule you prefer
        app.add_systems(OnEnter(Screen::Gameplay), spawn_timeline_ui);
        app.add_systems(Update, update_timeline_ui_via_event);
        app.add_systems(OnEnter(GamePhase::Day), reset_timeline_position);
    }
}

fn spawn_timeline_ui(mut commands: Commands) {
    // 1. Root Container Bar
    commands.spawn((
        Node {
            width: Val::Percent(80.0),            // Spans 80% of screen width
            height: Val::Px(24.0),                // Height of your timeline bar
            margin: UiRect::horizontal(Val::Auto), // Centers it on screen horizontally
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),                   // 20px padding from top edge
            border: UiRect::all(Val::Px(2.0)),    // 2px wide border line
            ..default()
        },
        BorderColor { top: Color::WHITE, right: Color::WHITE, bottom: Color::WHITE, left: Color::WHITE },
        BackgroundColor(Color::BLACK),
    )).with_children(|parent| {
        // 2. The Moving Vertical Line Indicator (Spawned as a child node)
        parent.spawn((
            Node {
                width: Val::Px(4.0),         // 4 pixels thick line ticker
                height: Val::Percent(100.0), // Fills container height completely
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),     // Starts pinned at the far left edge
                ..default()
            },
            BackgroundColor(Color::linear_rgb(1.0, 0.3, 0.3)), // Vibrant red line marker
            VisualTimelineTicker, // Tag component so our subscriber system can locate it
        ));
    });
}

/// This system reads layout messages frame-by-frame
fn update_timeline_ui_via_event(
    mut messages: MessageReader<NightTickEvent>, 
    mut query: Query<&mut Node, With<VisualTimelineTicker>>, // Note: Querying mut Node instead of Style!
) {
    // Safely look up our vertical bar's layout options
    if let Ok(mut ticker_node) = query.single_mut() {
        for event in messages.read() {
            // Linearly shift the bar's structural horizontal offset position
            let percent_left = event.fraction * 100.0;
            ticker_node.left = Val::Percent(percent_left);
        }
    }
}

/// Snaps the line cleanly back to the beginning on daybreak
fn reset_timeline_position(mut query: Query<&mut Node, With<VisualTimelineTicker>>) {
    if let Ok(mut node) = query.single_mut() {
        node.left = Val::Percent(0.0);
    }
}
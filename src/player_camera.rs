use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::AppState;

#[derive(Component)]
pub struct PlayerCamera {
    target_position: Vec2,
    target_scale: f32,
}

impl Default for PlayerCamera {
    fn default() -> Self {
        Self {
            target_position: Vec2::ZERO,
            target_scale: Self::INITIAL_SCALE,
        }
    }
}

impl PlayerCamera {
    const SPEED: f32 = 500.0;
    const POSITION_INTERPOLATION_FACTOR: f32 = 0.3;
    const INITIAL_SCALE: f32 = 1.0;
    const MIN_SCALE: f32 = 0.05;
    const MAX_SCALE: f32 = 1.0;
    const SCALE_STEP: f32 = 0.05;
    const SCALE_INTERPOLATION_FACTOR: f32 = 0.1;
}

fn setup(mut commands: Commands) {
    commands.spawn((
        PlayerCamera::default(),
        Camera2d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, PlayerCamera::INITIAL_SCALE)),
        GlobalTransform::default(),
        DespawnOnExit(AppState::Game),
    ));
}

fn controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<MouseWheel>,
    camera_query: Single<(&mut Transform, &mut PlayerCamera)>,
    time: Res<Time>,
) {
    let (mut transform, mut player_camera) = camera_query.into_inner();
    let delta_secs = time.delta_secs();

    // --- Zoom Controls ---
    let mut scale_delta = 0.0;
    for event in scroll_events.read() {
        scale_delta -= event.y;
    }

    if scale_delta != 0.0 {
        let new_target_scale = player_camera.target_scale + scale_delta * PlayerCamera::SCALE_STEP;
        player_camera.target_scale =
            new_target_scale.clamp(PlayerCamera::MIN_SCALE, PlayerCamera::MAX_SCALE);
    }

    // Smoothly interpolate camera zoom towards target zoom
    let current_zoom = transform.scale;
    let new_zoom = current_zoom.lerp(
        Vec3::splat(player_camera.target_scale),
        PlayerCamera::SCALE_INTERPOLATION_FACTOR,
    );
    transform.scale = new_zoom;

    // --- Movement Controls ---
    let mut direction = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction != Vec2::ZERO {
        let movement = direction.normalize() * PlayerCamera::SPEED * delta_secs;
        // Adjust movement speed based on zoom level
        let movement = movement * player_camera.target_scale;
        player_camera.target_position += movement;
    }

    // Smoothly interpolate camera position towards target position
    let current_position = Vec2::new(transform.translation.x, transform.translation.y);
    let new_position = current_position.lerp(
        player_camera.target_position,
        PlayerCamera::POSITION_INTERPOLATION_FACTOR,
    );
    transform.translation.x = new_position.x;
    transform.translation.y = new_position.y;
}

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), setup)
            .add_systems(Update, controls.run_if(in_state(AppState::Game)));
    }
}

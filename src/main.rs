use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource)]
struct Size(f32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Snake"),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Size(50.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (movement, change_direction))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component, Default, Clone)]
enum Direction {
    Up,
    Down,
    Left,
    #[default]
    Right,
}

#[derive(Component, Clone)]
struct LastDirection(Direction);

#[derive(Component)]
struct MovementTimer(Timer);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    size: Res<Size>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Player,
        Direction::default(),
        LastDirection(Direction::default()),
        MovementTimer(Timer::new(Duration::from_millis(100), TimerMode::Repeating)),
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(size.0)))),
        MeshMaterial2d(color_materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
}

fn movement(
    mut query: Query<
        (
            &mut MovementTimer,
            &mut Transform,
            &mut LastDirection,
            &Direction,
        ),
        With<Player>,
    >,
    time: Res<Time>,
    size: Res<Size>,
) {
    let (mut movement_timer, mut transform, mut last_direction, direction) = query.single_mut();
    movement_timer.0.tick(time.delta());

    if movement_timer.0.just_finished() {
        let offset = match direction {
            Direction::Up => (0.0, size.0),
            Direction::Down => (0.0, -size.0),
            Direction::Left => (-size.0, 0.0),
            Direction::Right => (size.0, 0.0),
        };
        transform.translation += Vec3::new(offset.0, offset.1, 0.0);
        last_direction.0 = direction.clone();
    }
}

fn change_direction(
    mut query: Query<(&mut Direction, &LastDirection), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let (mut direction, last_direction) = query.single_mut();

    let mut pressed_direction = Vec2::default();
    if keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        pressed_direction.y += 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        pressed_direction.y -= 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        pressed_direction.x -= 1.0;
    }
    if keys.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        pressed_direction.x += 1.0;
    }

    if matches!(last_direction.0, Direction::Left | Direction::Right) {
        *direction = match pressed_direction.y {
            1.0 => Direction::Up,
            -1.0 => Direction::Down,
            _ => direction.clone(),
        }
    } else {
        *direction = match pressed_direction.x {
            -1.0 => Direction::Left,
            1.0 => Direction::Right,
            _ => direction.clone(),
        }
    }
}

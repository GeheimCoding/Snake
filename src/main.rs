use bevy::prelude::*;
use rand::prelude::SliceRandom;
use std::time::Duration;

#[derive(Resource)]
struct Constants {
    size: f32,
    mesh_handle: Handle<Mesh>,
    color_handle: Handle<ColorMaterial>,
    apple_texture_handle: Handle<Image>,
}

#[derive(Event)]
struct MovementEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Snake"),
                ..default()
            }),
            ..default()
        }))
        .add_event::<MovementEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                trigger_movement,
                movement.run_if(on_event::<MovementEvent>),
                remove_tail.run_if(on_event::<MovementEvent>),
                change_direction,
            ),
        )
        .run();
}

#[derive(Component)]
struct Head;

#[derive(Component)]
struct Body;

#[derive(Component)]
struct Tail;

#[derive(Component)]
struct BodyPart;

#[derive(Component)]
struct NextBodyPart(Option<Entity>);

#[derive(Component)]
struct Apple;

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
    asset_server: Res<AssetServer>,
) {
    let size = 50.0;
    let speed = Duration::from_millis(100);
    let color = Color::srgb(0.3, 0.5, 0.3);
    let color_handle = color_materials.add(color);
    let mesh_handle = meshes.add(Rectangle::from_size(Vec2::splat(size)));

    let constants = Constants {
        size,
        mesh_handle: mesh_handle.clone(),
        color_handle: color_handle.clone(),
        apple_texture_handle: asset_server.load("textures/apple.png"),
    };

    commands.spawn(MovementTimer(Timer::new(speed, TimerMode::Repeating)));
    commands.spawn((Direction::default(), LastDirection(Direction::default())));

    let head_position = Vec2::default();
    let head = spawn_part(
        &mut commands,
        Head,
        head_position,
        &constants,
        NextBodyPart(None),
    );
    let body_position = Vec2::new(-size, 0.0);
    let body = spawn_part(
        &mut commands,
        Body,
        body_position,
        &constants,
        NextBodyPart(Some(head)),
    );
    let tail_position = Vec2::new(-2.0 * size, 0.0);
    spawn_part(
        &mut commands,
        Tail,
        tail_position,
        &constants,
        NextBodyPart(Some(body)),
    );

    spawn_apple(
        &mut commands,
        size,
        constants.apple_texture_handle.clone(),
        vec![head_position, body_position, tail_position],
    );

    commands.spawn(Camera2d);
    commands.insert_resource(constants);
}

fn trigger_movement(
    mut query: Query<&mut MovementTimer>,
    mut movement_event: EventWriter<MovementEvent>,
    time: Res<Time>,
) {
    if query.single_mut().0.tick(time.delta()).just_finished() {
        movement_event.send(MovementEvent);
    }
}

fn movement(
    mut commands: Commands,
    mut query: Query<(&mut LastDirection, &Direction)>,
    mut head_query: Query<(Entity, &mut Transform), With<Head>>,
    constants: Res<Constants>,
) {
    let size = constants.size;
    let (mut last_direction, direction) = query.single_mut();
    let (head, transform) = head_query.single_mut();
    let offset = Vec2::from(match direction {
        Direction::Up => (0.0, size),
        Direction::Down => (0.0, -size),
        Direction::Left => (-size, 0.0),
        Direction::Right => (size, 0.0),
    });

    let new_head = spawn_part(
        &mut commands,
        Head,
        transform.translation.truncate() + offset,
        &constants,
        NextBodyPart(None),
    );
    commands
        .entity(head)
        .remove::<Head>()
        .insert((Body, NextBodyPart(Some(new_head))));
    last_direction.0 = direction.clone();
}

fn change_direction(
    mut query: Query<(&mut Direction, &LastDirection)>,
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

fn remove_tail(mut commands: Commands, query: Query<(Entity, &NextBodyPart), With<Tail>>) {
    let (tail, next_part) = query.single();
    commands.entity(tail).despawn();
    commands
        .entity(next_part.0.expect("expected tail to have a next_part"))
        .remove::<Body>()
        .insert(Tail);
}

fn spawn_part<Part: Component>(
    commands: &mut Commands,
    part: Part,
    position: Vec2,
    constants: &Constants,
    next_part: NextBodyPart,
) -> Entity {
    commands
        .spawn((
            part,
            BodyPart,
            next_part,
            Mesh2d(constants.mesh_handle.clone()),
            MeshMaterial2d(constants.color_handle.clone()),
            Transform::from_xyz(position.x, position.y, 0.0),
        ))
        .id()
}

fn spawn_apple(
    commands: &mut Commands,
    size: f32,
    apple_texture: Handle<Image>,
    body_part_positions: Vec<Vec2>,
) {
    let mut spawn_points = Vec::new();
    for x in -11..11 {
        for y in -6..6 {
            spawn_points.push(Vec2::new(x as f32 * size, y as f32 * size));
        }
    }
    for position in body_part_positions {
        spawn_points.retain(|p| p.x != position.x && p.y != position.y);
    }

    spawn_points.shuffle(&mut rand::rng());
    commands.spawn((
        Apple,
        Sprite::from_image(apple_texture),
        Transform::from_translation(
            spawn_points
                .first()
                .expect("expected spawn point")
                .extend(0.0),
        ),
    ));
}

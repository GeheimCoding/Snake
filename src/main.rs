use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource)]
struct Constants {
    size: f32,
    mesh_handle: Handle<Mesh>,
    color_handle: Handle<ColorMaterial>,
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
struct NextPart(Option<Entity>);

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
    };

    commands.spawn(MovementTimer(Timer::new(speed, TimerMode::Repeating)));
    commands.spawn((Direction::default(), LastDirection(Direction::default())));

    let head = spawn_part(
        &mut commands,
        Head,
        Vec2::default(),
        &constants,
        NextPart(None),
    );
    let body = spawn_part(
        &mut commands,
        Body,
        Vec2::new(-size, 0.0),
        &constants,
        NextPart(Some(head)),
    );
    spawn_part(
        &mut commands,
        Tail,
        Vec2::new(-2.0 * size, 0.0),
        &constants,
        NextPart(Some(body)),
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
        NextPart(None),
    );
    commands
        .entity(head)
        .remove::<Head>()
        .insert((Body, NextPart(Some(new_head))));
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

fn spawn_part<Part: Component>(
    commands: &mut Commands,
    part: Part,
    position: Vec2,
    constants: &Constants,
    next_part: NextPart,
) -> Entity {
    commands
        .spawn((
            part,
            next_part,
            Mesh2d(constants.mesh_handle.clone()),
            MeshMaterial2d(constants.color_handle.clone()),
            Transform::from_xyz(position.x, position.y, 0.0),
        ))
        .id()
}

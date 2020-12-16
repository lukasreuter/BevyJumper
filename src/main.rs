use crate::components::{
    Airborne, ButtonEvent, DashCooldown, Direction, GameplayInputs, LookDirection, Movement,
};
use bevy::app::App;
use bevy::asset::Assets;
use bevy::core::Time;
use bevy::ecs::{Commands, Entity, Query, Res, ResMut, Without};
use bevy::input::{keyboard::KeyCode, Input, system::exit_on_esc_system};
use bevy::math::Vec2;
use bevy::render::{
    color::Color, entity::Camera2dBundle, pass::ClearColor, render_graph::base::Msaa,
};
use bevy::sprite::{entity::SpriteBundle, ColorMaterial, Sprite};
use bevy::window::WindowDescriptor;
use bevy::DefaultPlugins;
use bevy_rapier2d::physics::{
    EventQueue, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent,
};
use bevy_rapier2d::rapier::dynamics::{RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use bevy_rapier2d::rapier::na::Vector2;
use bevy_rapier2d::rapier::ncollide::pipeline::ContactEvent;
use bevy_rapier2d::rapier::pipeline::PhysicsPipeline;
use bevy_rapier2d::render::RapierRenderPlugin;

mod components;

fn update_dash_cooldown(
    commands: &mut Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DashCooldown)>,
) {
    for (entity, mut cooldown) in query.iter_mut() {
        cooldown.0.tick(time.delta_seconds());

        if cooldown.0.just_finished() {
            commands.remove_one::<DashCooldown>(entity);
        }
    }
}

fn keyboard_input(keyboard: Res<Input<KeyCode>>, mut inputs: Query<&mut GameplayInputs>) {
    for mut input in inputs.iter_mut() {
        const L1: KeyCode = KeyCode::A;
        const L2: KeyCode = KeyCode::Left;
        const R1: KeyCode = KeyCode::D;
        const R2: KeyCode = KeyCode::Right;
        const J: KeyCode = KeyCode::Space;

        input.move_left = ButtonEvent {
            down: keyboard.pressed(L1) || keyboard.pressed(L2),
            pressed_this_frame: keyboard.pressed(L1) || keyboard.pressed(L2),
            released_this_frame: keyboard.pressed(L1) || keyboard.pressed(L2),
        };
        input.move_right = ButtonEvent {
            down: keyboard.pressed(R1) || keyboard.pressed(R2),
            pressed_this_frame: keyboard.pressed(R1) || keyboard.pressed(R2),
            released_this_frame: keyboard.pressed(R1) || keyboard.pressed(R2),
        };
        input.jump = ButtonEvent {
            down: keyboard.pressed(J),
            pressed_this_frame: keyboard.pressed(J),
            released_this_frame: keyboard.pressed(J),
        }
    }
}

fn player_movement(
    commands: &mut Commands,
    mut rigid_bodies: ResMut<RigidBodySet>,
    mut player_info: Query<
            (
                Entity,
                &Movement,
                &mut Direction,
                &GameplayInputs,
                &RigidBodyHandleComponent,
            ),
            Without<Airborne>,
    >,
) {
    for (entity, movement, mut direction, gameplay_inputs, rigid_body_component) in
        player_info.iter_mut()
    {
        if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
            if gameplay_inputs.move_right.down {
                direction.value = LookDirection::Right;

                // scale.Value.x = abs(scale.Value.x);

                let lin_vel = *rb.linvel();
                let new_vel_x = f32::min(
                    lin_vel.x + movement.horizontal_acceleration,
                    movement.max_speed,
                );
                rb.set_linvel(Vector2::new(new_vel_x, lin_vel.y), true);
            } else if gameplay_inputs.move_left.down {
                direction.value = LookDirection::Left;

                // scale.Value.x = -abs(scale.Value.x);

                let lin_vel = *rb.linvel();
                let new_vel_x = f32::max(
                    lin_vel.x - movement.horizontal_acceleration,
                    -movement.max_speed,
                );
                rb.set_linvel(Vector2::new(new_vel_x, lin_vel.y), true);
            } else {
                let lin_vel = *rb.linvel();
                rb.set_linvel(Vector2::new(0.0, lin_vel.y), true);
            }

            if gameplay_inputs.jump.pressed_this_frame {
                let lin_vel = *rb.linvel();
                rb.set_linvel(Vector2::new(lin_vel.x, movement.jump_power), true);
                // gravity.Scale = movement.RisingGravityScale;
                commands.insert_one(
                    entity,
                    Airborne {
                        direction: direction.value,
                        reached_jump_apex: false,
                    },
                );
            }
        }
    }
}

fn player_air_movement(
    mut rigid_bodies: ResMut<RigidBodySet>,
    mut player_info: Query<(
        &Airborne,
        &Movement,
        &GameplayInputs,
        &RigidBodyHandleComponent,
    )>,
) {
    for (airborne, movement, gameplay_inputs, rigid_body_component) in player_info.iter_mut() {
        if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
            if gameplay_inputs.move_right.down {
                let max_speed = get_air_max_speed(LookDirection::Right, airborne, movement);
                let accel = movement.horizontal_acceleration;
                let lin_vel = *rb.linvel();
                let new_vel_x = f32::min(lin_vel.x + accel, max_speed);
                rb.set_linvel(Vector2::new(new_vel_x, lin_vel.y), true);
            } else if gameplay_inputs.move_left.down {
                let max_speed = get_air_max_speed(LookDirection::Left, airborne, movement);
                let accel = movement.horizontal_acceleration;
                let lin_vel = *rb.linvel();
                let new_vel_x = f32::max(lin_vel.x - accel, -max_speed);
                rb.set_linvel(Vector2::new(new_vel_x, lin_vel.y), true);
            } else {
                let lin_vel = *rb.linvel();
                rb.set_linvel(Vector2::new(0.0, lin_vel.y), true);
            }
        }
    }
}

fn grounding(
    commands: &mut Commands,
    events: Res<EventQueue>,
    players: Query<(Entity, &Airborne, &RigidBodyHandleComponent)>,
) {
    while let Ok(contact_event) = events.contact_events.pop() {
        println!("Received contact event: {:?}", contact_event);

        match contact_event {
            ContactEvent::Started(c1, c2) => {
                for (entity, _airborne, rigid_body_component) in players.iter() {
                    if rigid_body_component.handle() == c1 || rigid_body_component.handle() == c2 {
                        commands.remove_one::<Airborne>(entity);
                        println!("detected ground");
                    }
                }
            }
            _ => {}
        }
    }
}

fn apex_detection(
    rigid_bodies: Res<RigidBodySet>,
    mut airborne_players: Query<(&mut Airborne, &RigidBodyHandleComponent)>,
) {
    for (mut airborne, rigid_body_handle) in airborne_players.iter_mut() {
        // if have not reached the apex yet but our velocity is negative (going down to the ground)
        // then that means we are not rising anymore and reached the jump apex
        if let Some(rb) = rigid_bodies.get(rigid_body_handle.handle()) {
            let lin_vel = rb.linvel();
            if !airborne.reached_jump_apex && lin_vel.y < 0.0 {
                airborne.reached_jump_apex = true;
            }
        }
    }
}

fn get_air_max_speed(
    input_direction: LookDirection,
    airborne: &Airborne,
    movement: &Movement,
) -> f32 {
    match (
        movement.commit_jump_direction,
        airborne.direction == input_direction,
    ) {
        (true, true) => movement.air_forward_max_speed,
        (true, false) => movement.air_backward_max_speed,
        (false, _) => movement.max_speed,
    }
}

fn startup_system(
    commands: &mut Commands,
    mut conf: ResMut<RapierConfiguration>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // increased the scale for faster descent
    let scale = 15.0;

    conf.scale = scale;

    let player_size_x = 40.0;
    let player_size_y = 80.0;

    let ground_size = 25.0;
    let rigid_body = RigidBodyBuilder::new_static()
        .translation(0.0, -ground_size)
        .lock_rotations();
    let collider = ColliderBuilder::cuboid(ground_size, 1.2);

    // Spawn entity with `Player` struct as a component for access in movement query.
    commands
        // camera
        .spawn(Camera2dBundle::default())
        // ground
        .spawn((rigid_body, collider))
        // player
        .spawn(SpriteBundle {
            material: materials.add(Color::rgb(0.45, 0.46, 1.0).into()),
            sprite: Sprite::new(Vec2::new(player_size_x, player_size_y)),
            ..Default::default()
        })
        .with(Movement {
            max_speed: (20.0),
            horizontal_acceleration: (5.0),
            jump_power: (10.0),
            air_forward_max_speed: (15.0),
            air_backward_max_speed: (7.0),
            rising_gravity_scale: (1.0),
            falling_gravity_scale: (3.0),
            commit_jump_direction: (true),
        })
        .with(GameplayInputs {
            ..Default::default()
        })
        .with(Direction {
            value: LookDirection::Right,
        })
        .with(
            RigidBodyBuilder::new_dynamic()
                .mass(300.0, false)
                .lock_rotations(),
        )
        .with(ColliderBuilder::cuboid(
            player_size_x / 2.0 / scale,
            player_size_y / 2.0 / scale,
        ));
}

fn enable_physics_profiling(mut pipeline: ResMut<PhysicsPipeline>) {
    pipeline.counters.enable()
}

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Bevy Jumper".to_string(),
            width: 1000.0,
            height: 1000.0,
            ..Default::default()
        })
        .add_resource(ClearColor(Color::rgb(0.19, 0.30, 0.47)))
        .add_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_startup_system(startup_system)
        .add_startup_system(enable_physics_profiling)
        .add_system(exit_on_esc_system)
        .add_system(update_dash_cooldown)
        .add_system(keyboard_input)
        .add_system(player_movement)
        .add_system(player_air_movement)
        .add_system(grounding)
        .add_system(apex_detection)
        .run();
}

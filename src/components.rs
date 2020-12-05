use bevy::core::Timer;

pub struct Movement {
    pub max_speed: f32,
    pub horizontal_acceleration: f32,
    pub jump_power: f32,
    pub air_forward_max_speed: f32,
    pub air_backward_max_speed: f32,
    pub rising_gravity_scale: f32,
    pub falling_gravity_scale: f32,
    pub commit_jump_direction: bool,
}

#[derive(Default, Debug)]
pub struct ButtonEvent {
    pub down: bool,
    pub pressed_this_frame: bool,
    pub released_this_frame: bool,
}

#[derive(Default, Debug)]
pub struct GameplayInputs {
    pub move_left: ButtonEvent,
    pub move_right: ButtonEvent,
    pub jump: ButtonEvent,
}

pub struct DashCooldown(pub Timer);

pub struct Airborne {
    pub direction: LookDirection,
    pub reached_jump_apex: bool,
}

pub struct Direction {
    pub value: LookDirection,
}

#[derive(PartialEq, Copy, Clone)]
pub enum LookDirection {
    Left,
    Right,
}

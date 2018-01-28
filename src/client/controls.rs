use std;
use std::collections::VecDeque;
use std::collections::HashMap;

use super::glium::glutin;
use self::glutin::ElementState;
use self::glutin::VirtualKeyCode;
use self::glutin::MouseButton;
use self::glutin::DeviceId;
use self::glutin::KeyboardInput;
use self::glutin::ModifiersState;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum State {
    Active,
    Inactive,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum StateTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
    Jump,
    Exit,
    ToggleMenu,
}

#[derive(Debug, Copy, Clone)]
pub enum AxisTarget {
    Yaw,
    Pitch,
}

#[derive(Debug)]
struct AxisMapping {
    target: AxisTarget,
    inverted: bool,
}

#[derive(Debug)]
pub enum InputEvent {
    StateEvent { target: StateTarget, state: State },
    AxisEvent { target: AxisTarget, value: f64},
}

pub struct Controls {
    input_events: VecDeque<InputEvent>,
    states: HashMap<StateTarget, State>,
    // TODO mouse wheel
    scan_code_mapping: HashMap<u32, StateTarget>, // TODO allow multiple targets
    key_code_mapping: HashMap<VirtualKeyCode, StateTarget>, // TODO allow multiple targets
    button_mapping: HashMap<MouseButton, StateTarget>, // TODO allow multiple targets
    axis_mapping: HashMap<u32, AxisMapping>, // TODO allow multiple targets
}

impl Controls {
    pub fn process_motion_event(&mut self, _device_id: DeviceId, axis: u32, mut value: f64 ) {
        use self::InputEvent::*;
        if let Some(bind) = self.axis_mapping.get(&axis) {
            if bind.inverted {
                value = -value;
            }
            self.input_events.push_back(AxisEvent { target: bind.target, value });
        }
    }

    pub fn process_keyboard_input_event(&mut self, _device_id: DeviceId, input: KeyboardInput ) {
        let mut targets = Vec::new();

        if let Some(&target) = self.scan_code_mapping.get(&input.scancode) {
            targets.push(target);
        }
        if let Some(key_code) = input.virtual_keycode {
            if let Some(&target) = self.key_code_mapping.get(&key_code) {
                targets.push(target);
            }
        }
        self.set_state_targets(targets, input.state);
    }

    pub fn process_mouse_input_event(&mut self, _device_id: DeviceId, state: ElementState,
                                     button: MouseButton, _modifiers: ModifiersState ) {
        let mut targets = Vec::new();

        if let Some(&target) = self.button_mapping.get(&button) {
            targets.push(target);
        }
        self.set_state_targets(targets, state);
    }

    fn set_state_targets(&mut self, targets: Vec<StateTarget>, element_state: ElementState) {
        use self::ElementState::*;
        use self::State::*;
        use self::InputEvent::*;

        let state = match element_state {
            Pressed => Active,
            Released => Inactive,
        };
        for target in targets {
            let mut changed;
            match self.states.insert(target, state) {
                Some(old_state) => changed = old_state != state,
                None => changed = true,
            }
            if changed {
                self.input_events.push_back(StateEvent { target, state });
            }
        }
    }

    pub fn get_events(&mut self) -> Vec<InputEvent> {
        let mut events = VecDeque::new();
        std::mem::swap(&mut events, &mut self.input_events);
        events.into()
    }

    pub fn get_state(&self, target: StateTarget) -> State {
        *self.states.get(&target).unwrap_or(&State::Inactive)
    }
}

impl Default for Controls {
    fn default() -> Self {
        use self::StateTarget::*;
        use self::AxisTarget::*;
        Controls {
            input_events: VecDeque::new(),
            states: HashMap::new(),
            scan_code_mapping: vec!(
                (17, MoveForward),
                (31, MoveBackward),
                (30, MoveLeft),
                (32, MoveRight),
                (57, Jump),
            ).into_iter().collect(),
            key_code_mapping: vec!(
                (VirtualKeyCode::Q, Exit),
                (VirtualKeyCode::Escape, ToggleMenu),
            ).into_iter().collect(),
            button_mapping: vec!().into_iter().collect(),
            axis_mapping: vec!(
                (0, AxisMapping { target: Yaw, inverted: true}),
                (1, AxisMapping { target: Pitch, inverted: true}),
            ).into_iter().collect(),
        }
    }
}
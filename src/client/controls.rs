use std::collections::VecDeque;
use std::collections::HashMap;

use super::glium::glutin;
use self::glutin::ElementState;

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
}

#[derive(Debug, Copy, Clone)]
enum ButtonOrKey {
    Button(u32),
    Key(u32),
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

pub struct InputEventIterator<'a> {
    controls: &'a mut Controls,
}

impl<'a> Iterator for InputEventIterator<'a> {
    type Item = InputEvent;
    fn next(&mut self) -> Option<Self::Item> {
        self.controls.input_events.pop_front()
    }
}

pub struct Controls {
    input_events: VecDeque<InputEvent>,
    states: HashMap<StateTarget, State>,
    key_mapping: HashMap<u32, StateTarget>,
    button_mapping: HashMap<u32, StateTarget>,
    axis_mapping: HashMap<u32, AxisMapping>,
}

impl Controls {
    pub fn process_device_event(&mut self, _id: glutin::DeviceId, event: glutin::DeviceEvent) {
        use self::glutin::DeviceEvent as DE;
        use self::InputEvent::*;
        use self::ButtonOrKey::*;
        //println!("{:?}", event);
        match event {
            DE::Added => println!("Device added"),
            DE::Removed => println!("Device removed"),
            // MouseMotion unit is sensor(?) unit, not pixels!
            // There will also be a motion event for the axis
            DE::MouseMotion {delta: _d} => (),
            DE::MouseWheel {delta: _d} => (),
            // Motion unit is sensor(?) unit, not pixels!
            DE::Motion { axis, mut value } => {
                if let Some(bind) = self.axis_mapping.get(&axis) {
                    if bind.inverted {
                        value = -value;
                    }
                    self.input_events.push_back(AxisEvent { target: bind.target, value });
                }
            },
            DE::Button {button, state} => self.handle_button_or_key(Button(button), state),
            // Key only occurs on state change, no repetition
            DE::Key(ki) => self.handle_button_or_key(Key(ki.scancode), ki.state),
            DE::Text {codepoint: c} => println!("Text: {}", c),
        }
    }

    pub fn events_iter(&mut self) -> InputEventIterator {
        InputEventIterator {controls: self}
    }

    pub fn get_state(&self, target: StateTarget) -> State {
        *self.states.get(&target).unwrap_or(&State::Inactive)
    }

    fn handle_button_or_key(&mut self, bind: ButtonOrKey, element_state: ElementState) {
        use self::ButtonOrKey::*;
        use self::State::*;
        use self::ElementState::*;
        use self::InputEvent::StateEvent;

        let map;
        let key;
        match bind {
            Key(k) => {
                map = &self.key_mapping;
                key = k;
            },
            Button(k) => {
                map = &self.button_mapping;
                key = k;
            },
        }
        if let Some(target) = map.get(&key) {
            let state = match element_state {
                Pressed => Active,
                Released => Inactive,
            };
            let mut changed;
            match self.states.insert(*target, state) {
                Some(old_state) => changed = old_state != state,
                None => changed = true,
            }
            if changed {
                self.input_events.push_back(StateEvent { target: *target, state });
            }
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        use self::StateTarget::*;
        use self::AxisTarget::*;
        Controls {
            input_events: VecDeque::new(),
            states: HashMap::new(),
            key_mapping: vec!(
                (17, MoveForward),
                (31, MoveBackward),
                (30, MoveLeft),
                (32, MoveRight),
                (57, Jump),
            ).into_iter().collect(),
            button_mapping: vec!().into_iter().collect(),
            axis_mapping: vec!(
                (0, AxisMapping { target: Yaw, inverted: true}),
                (1, AxisMapping { target: Pitch, inverted: true}),
            ).into_iter().collect(),
        }
    }
}
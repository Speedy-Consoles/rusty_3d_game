mod targets;
mod triggers;
mod push_button;

extern crate num;

use std;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::BTreeSet;

use self::num::cast::NumCast;
use super::toml;
use super::glium::glutin;
use self::glutin::ElementState;
use self::glutin::VirtualKeyCode;
use self::glutin::MouseScrollDelta;
use self::glutin::TouchPhase;
use self::glutin::DeviceId;
use self::glutin::KeyboardInput;
use self::glutin::ModifiersState;

pub use self::targets::*;
pub use self::triggers::*;
pub use self::push_button::*;

#[derive(Debug)]
pub struct ParseError(String);

#[derive(Debug, PartialEq)]
pub enum PushButtonState {
    Pressed,
    Released,
}

#[derive(Debug, PartialEq)]
pub enum MouseWheelDirection {
    Up,
    Down,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SwitchState {
    Active,
    Inactive,
}

#[derive(Debug, Default)]
struct PushButtonMapping {
    on_press: BTreeSet<FireTarget>,
    while_down: BTreeSet<SwitchTarget>,
}

#[derive(Debug, Default)]
struct MouseWheelMapping {
    on_up: BTreeSet<FireTarget>,
    on_down: BTreeSet<FireTarget>,
    on_change: BTreeSet<ValueTarget>,
}

type AxisMapping = BTreeSet<ValueTarget>;

#[derive(Debug)]
pub enum ControlEvent {
    Fire(FireTarget),
    Switch { target: SwitchTarget, state: SwitchState },
    Value { target: ValueTarget, value: f64 },
}

#[derive(Debug)]
pub enum Bind {
    Fire(FireTrigger, FireTarget),
    Switch(SwitchTrigger, SwitchTarget),
    Value(ValueTrigger, ValueTarget),
}

pub struct Controls {
    events: VecDeque<ControlEvent>,
    switch_counter: HashMap<SwitchTarget, u32>,
    push_button_mappings: HashMap<PushButton, PushButtonMapping>,
    axis_mappings: HashMap<u32, AxisMapping>,
    mouse_wheel_mapping: MouseWheelMapping,
    value_factors: HashMap<ValueTarget, f64>,
    last_key_state: HashMap<u32, ElementState>,
    last_button_state: HashMap<glutin::MouseButton, ElementState>,
}

impl Controls {
    fn new() -> Controls {
        Controls {
            events: VecDeque::new(),
            switch_counter: HashMap::new(),
            push_button_mappings: HashMap::new(),
            axis_mappings: HashMap::new(),
            mouse_wheel_mapping: Default::default(),
            value_factors: HashMap::new(),
            last_key_state: HashMap::new(),
            last_button_state: HashMap::new(),
        }
    }

    pub fn from_toml(value: &toml::value::Value) -> Result<Controls, ParseError> {
        use self::Bind::*;
        use self::toml::value::Value::Table;
        use self::toml::value::Value::Float;

        let mut controls = Controls::new();
        let table = match value {
            &Table(ref t) => t,
            _ => return Err(ParseError(String::from("Controls must be a table!"))),
        };

        match table.get("binds") {
            Some(v) => match v {
                &Table(ref keys) => for (target_string, trigger_value) in keys {
                    let bind = match target_string.parse()? {
                        Target::Fire(target) =>
                            Fire(FireTrigger::from_toml(trigger_value)?, target),
                        Target::Switch(target) =>
                            Switch(SwitchTrigger::from_toml(trigger_value)?, target),
                        Target::Value(target) =>
                            Value(ValueTrigger::from_toml(trigger_value)?, target),
                    };
                    controls.add_bind(bind);
                },
                _ => return Err(ParseError(String::from("Binds must be a table!"))),
            },
            None => return Err(ParseError(String::from("No binds section found in controls!"))),
        }
        match table.get("factors") {
            Some(v) => match v {
                &Table(ref factors) => for (target_string, trigger_value) in factors {
                    match target_string.parse()? {
                        Target::Value(target) => match trigger_value {
                            &Float(factor) => controls.set_factor(target, factor),
                            v => return Err(ParseError(
                                format!("Factor must be a float, got '{}'!", v)
                            )),
                        }
                        _ => return Err(ParseError(format!("Expected value target!"))),
                    };
                },
                _ => return Err(ParseError(String::from("Binds must be a table!"))),
            },
            None => return Err(ParseError(String::from("No binds section found in controls!"))),
        }
        Ok(controls)
    }

    pub fn set_factor(&mut self, target: ValueTarget, factor: f64) {
        self.value_factors.insert(target, factor);
    }

    pub fn add_bind(&mut self, bind: Bind) {
        use self::Bind::*;

        match bind {
            Fire(trigger, target) => self.set_fire_target_trigger(trigger, target),
            Switch(trigger, target) => self.set_switch_target_trigger(trigger, target),
            Value(trigger, target) => self.set_value_target_trigger(trigger, target),
        };
    }

    fn set_fire_target_trigger(&mut self, trigger: FireTrigger, target: FireTarget) {
        use self::FireTrigger::*;
        use self::PushButtonState::*;
        use self::MouseWheelDirection::*;

        self.remove_fire_target_trigger(target);
        match trigger {
            Button(SwitchTrigger { button, state }) => {
                let mut mapping = self.push_button_mappings.entry(button)
                    .or_insert_with(Default::default);
                if state == Pressed {
                    mapping.on_press.insert(target);
                }
            },
            MouseWheelTick(direction) => {
                let mapping = &mut self.mouse_wheel_mapping;
                match direction {
                    Up => mapping.on_up.insert(target),
                    Down => mapping.on_down.insert(target),
                };
            }
        };
    }

    fn set_switch_target_trigger(&mut self, trigger: SwitchTrigger, target: SwitchTarget) {
        use self::PushButtonState::*;

        self.remove_switch_target_trigger(target);
        let mapping = self.push_button_mappings.entry(trigger.button)
            .or_insert_with(Default::default);
        if trigger.state == Pressed {
            mapping.while_down.insert(target);
        };
    }

    fn set_value_target_trigger(&mut self, trigger: ValueTrigger, target: ValueTarget) {
        use self::ValueTrigger::*;
        self.remove_value_target_trigger(target);
        match trigger {
            MouseX => {
                // TODO
            },
            MouseY => {
                // TODO
            },
            MouseWheel => {
                self.mouse_wheel_mapping.on_change.insert(target);
            },
            Axis(axis) => {
                self.axis_mappings.entry(axis).or_insert_with(Default::default).insert(target);
            },
        };
    }

    pub fn remove_fire_target_trigger(&mut self, target: FireTarget) {
        for (_, mapping) in &mut self.push_button_mappings {
            if mapping.on_press.remove(&target) {
                return
            }
        }
        if self.mouse_wheel_mapping.on_up.remove(&target) {
            return
        }
        if self.mouse_wheel_mapping.on_down.remove(&target) {
            return
        }
    }

    pub fn remove_switch_target_trigger(&mut self, target: SwitchTarget) {
        self.switch_counter.insert(target, 0);
        for (_, mapping) in &mut self.push_button_mappings {
            if mapping.while_down.remove(&target) {
                return
            }
        }
    }

    pub fn remove_value_target_trigger(&mut self, target: ValueTarget) {
        for (_, mapping) in &mut self.axis_mappings {
            if mapping.remove(&target) {
                return
            }
        }

        if self.mouse_wheel_mapping.on_change.remove(&target) {
            return
        }
    }

    pub fn get_events(&mut self) -> Vec<ControlEvent> {
        let mut events = VecDeque::new();// TODO get rid of allocation
        std::mem::swap(&mut events, &mut self.events);
        events.into()
    }

    pub fn get_state(&self, target: SwitchTarget) -> SwitchState {
        use self::SwitchState::*;
        match *self.switch_counter.get(&target).unwrap() {
            0 => Inactive,
            _ => Active,
        }
    }

    pub fn process_motion_event(&mut self, _device_id: DeviceId, axis: u32, mut value: f64) {
        use self::ControlEvent::*;
        if let Some(mapping) = self.axis_mappings.get(&axis) {
            for &target in mapping {
                let factor = self.value_factors.get(&target).unwrap_or(&1.0);
                value *= factor * target.get_base_factor();
                self.events.push_back(Value { target, value });
            }
        }
    }

    pub fn process_keyboard_input_event(&mut self, _device_id: DeviceId, input: KeyboardInput) {
        use self::PushButton::*;

        let last_state = self.last_key_state.insert(input.scancode, input.state)
            .unwrap_or(ElementState::Released);
        if last_state == input.state {
            return;
        }
        if let Some(key_code) = input.virtual_keycode {
            self.set_push_button_targets(KeyCode(key_code), input.state);
        }
        self.set_push_button_targets(ScanCode(input.scancode), input.state);
    }

    pub fn process_mouse_input_event(&mut self, _device_id: DeviceId, state: ElementState,
                                     button: glutin::MouseButton, _modifiers: ModifiersState) {
        use self::PushButton::*;

        let last_state = self.last_button_state.insert(button, state)
            .unwrap_or(ElementState::Released);
        if last_state == state {
            return;
        }
        self.set_push_button_targets(MouseButton(button), state);
    }

    pub fn process_mouse_wheel_event(&mut self, _device_id: DeviceId, delta: MouseScrollDelta,
                                     _phase: TouchPhase, _modifiers: ModifiersState) {
        use self::MouseScrollDelta::*;
        use self::ControlEvent::*;

        let value = match delta { // TODO also handle x and PixelDelta?
            LineDelta(_x, y) => y as f64,
            PixelDelta(_x, _y) => return,
        };

        if value > 0.0 {
            for &fire_target in self.mouse_wheel_mapping.on_up.iter() {
                self.events.push_back(Fire(fire_target));
            }
        } else if value < 0.0 {
            for &fire_target in self.mouse_wheel_mapping.on_down.iter() {
                self.events.push_back(Fire(fire_target));
            }
        }
        for &target in self.mouse_wheel_mapping.on_change.iter() {
            self.events.push_back(Value { target, value: value });
        }
    }

    fn set_push_button_targets(&mut self, push_button: PushButton, state: ElementState) {
        use self::ElementState::*;
        use self::SwitchState::*;
        use self::ControlEvent::*;

        if let Some(mapping) = self.push_button_mappings.get_mut(&push_button) {
            if state == Pressed {
                for &fire_target in mapping.on_press.iter() {
                    self.events.push_back(Fire(fire_target));
                }
            }
            for &switch_target in mapping.while_down.iter() {
                let counter = self.switch_counter.get_mut(&switch_target).unwrap();
                if *counter == 0 {
                    self.events.push_back(Switch { target: switch_target, state: Active });
                }
                match state {
                    Pressed => *counter += 1,
                    Released => *counter -= 1,
                }
                if *counter == 0 {
                    self.events.push_back(Switch { target: switch_target, state: Inactive });
                }
            }
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        use self::PushButtonState::*;
        use self::FireTarget::*;
        use self::SwitchTarget::*;
        use self::ValueTarget::*;
        use self::PushButton::*;
        use self::FireTrigger::*;
        use self::ValueTrigger::*;
        use self::MouseWheelDirection::*;
        use self::glutin::MouseButton::*;
        use self::Bind::*;

        let binds = vec!(
            Switch(SwitchTrigger { button: ScanCode(17), state: Pressed }, MoveForward),
            Switch(SwitchTrigger { button: ScanCode(31), state: Pressed }, MoveBackward),
            Switch(SwitchTrigger { button: ScanCode(30), state: Pressed }, MoveLeft),
            Switch(SwitchTrigger { button: ScanCode(32), state: Pressed }, MoveRight),
            Switch(SwitchTrigger { button: MouseButton(Left), state: Pressed }, Shoot),
            Switch(SwitchTrigger { button: MouseButton(Right), state: Pressed }, Aim),
            Fire(Button(SwitchTrigger { button: ScanCode(57), state: Pressed }), Jump),
            Fire(Button(SwitchTrigger { button: KeyCode(VirtualKeyCode::Q), state: Pressed }),
                 Exit),
            Fire(Button(SwitchTrigger { button: KeyCode(VirtualKeyCode::Escape), state: Pressed }),
                 ToggleMenu),
            Fire(MouseWheelTick(Up), PrevWeapon),
            Fire(MouseWheelTick(Down), NextWeapon),
            Value(Axis(0), Yaw),
            Value(Axis(1), Pitch),
        );
        let mut controls = Controls::new();
        for bind in binds {
            controls.add_bind(bind);
        }
        controls
    }
}
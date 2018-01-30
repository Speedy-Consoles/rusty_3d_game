use std;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::glium::glutin;
use self::glutin::ElementState;
use self::glutin::VirtualKeyCode;
use self::glutin::MouseButton;
use self::glutin::MouseScrollDelta;
use self::glutin::TouchPhase;
use self::glutin::DeviceId;
use self::glutin::KeyboardInput;
use self::glutin::ModifiersState;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum SwitchState {
    Active,
    Inactive,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FireTarget {
    Jump,
    Exit,
    ToggleMenu,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwitchTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueTarget {
    Yaw,
    Pitch,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum PushButton {
    ScanCode(u32),
    KeyCode(VirtualKeyCode),
    Button(MouseButton),
}

#[derive(Debug, Default)]
struct PushButtonBinds {
    on_press: BTreeSet<FireTarget>,
    on_release: BTreeSet<FireTarget>,
    while_down: BTreeSet<SwitchTarget>,
}

type AxisBinds = BTreeMap<ValueTarget, f64>;

#[derive(Debug, Default)]
struct MouseWheelBinds {
    on_positive: BTreeSet<FireTarget>,
    on_negative: BTreeSet<FireTarget>,
    on_change: BTreeMap<ValueTarget, f64>,
}

#[derive(Debug)]
pub enum ControlEvent {
    Fire(FireTarget),
    Switch { target: SwitchTarget, state: SwitchState },
    Value { target: ValueTarget, value: f64 },
}

#[derive(Debug)]
pub enum Bind {
    OnPress(PushButton, FireTarget),
    OnRelease(PushButton, FireTarget),
    WhileDown(PushButton, SwitchTarget),
    ForAxis(u32, f64, ValueTarget),
    OnMouseWheelUp(FireTarget),
    OnMouseWheelDown(FireTarget),
    ForMouseWheel(f64, ValueTarget),
}

pub struct Controls {
    events: VecDeque<ControlEvent>,
    switch_counter: HashMap<SwitchTarget, u32>,
    push_button_mapping: HashMap<PushButton, PushButtonBinds>,
    axis_mapping: HashMap<u32, AxisBinds>,
    mouse_wheel_mapping: MouseWheelBinds,
    last_key_state: HashMap<u32, ElementState>,
    last_button_state: HashMap<MouseButton, ElementState>,
}

impl Controls {
    fn from_binds(binds: Vec<Bind>) -> Controls {
        use self::Bind::*;

        let mut switch_counter = HashMap::new();

        let mut push_button_mapping: HashMap<PushButton, PushButtonBinds> = HashMap::new();
        let mut axis_mapping: HashMap<u32, AxisBinds> = HashMap::new();
        let mut mouse_wheel_mapping: MouseWheelBinds = Default::default();
        for bind in binds {
            let success = match &bind {
                &OnPress(button, target) => push_button_mapping.entry(button).or_default()
                    .on_press.insert(target),
                &OnRelease(button, target) => push_button_mapping.entry(button).or_default()
                    .on_release.insert(target),
                &WhileDown(button, target) => {
                    switch_counter.insert(target, 0);
                    push_button_mapping.entry(button).or_default().while_down.insert(target)
                },
                &ForAxis(id, factor, target) => axis_mapping.entry(id).or_default()
                    .insert(target, factor).is_none(),
                &OnMouseWheelUp(target) => mouse_wheel_mapping.on_positive.insert(target),
                &OnMouseWheelDown(target) => mouse_wheel_mapping.on_negative.insert(target),
                &ForMouseWheel(factor, target) => mouse_wheel_mapping.on_change
                    .insert(target, factor).is_none(),
            };
            if !success {
                println!("Duplicate bind:\n {:?}", bind);
            }
        }
        Controls::new(push_button_mapping, axis_mapping, mouse_wheel_mapping)
    }

    fn new(push_button_mapping: HashMap<PushButton, PushButtonBinds>,
           axis_mapping: HashMap<u32, AxisBinds>,
           mouse_wheel_mapping: MouseWheelBinds) -> Controls {
        let mut switch_counter = HashMap::new();
        for binds in push_button_mapping.values() {
            for &target in binds.while_down.iter() {
                switch_counter.insert(target, 0);
            }
        }

        Controls {
            events: VecDeque::new(),
            switch_counter,
            push_button_mapping,
            axis_mapping,
            mouse_wheel_mapping,
            last_key_state: HashMap::new(),
            last_button_state: HashMap::new(),
        }
    }

    pub fn get_events(&mut self) -> Vec<ControlEvent> {
        let mut events = VecDeque::new();
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
        if let Some(value_binds) = self.axis_mapping.get(&axis) {
            for (&target, &factor) in value_binds {
                value *= factor;
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
                                     button: MouseButton, _modifiers: ModifiersState) {
        use self::PushButton::*;

        let last_state = self.last_button_state.insert(button, state)
            .unwrap_or(ElementState::Released);
        if last_state == state {
            return;
        }
        self.set_push_button_targets(Button(button), state);
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
            for &fire_target in self.mouse_wheel_mapping.on_positive.iter() {
                self.events.push_back(Fire(fire_target));
            }
        } else if value < 0.0 {
            for &fire_target in self.mouse_wheel_mapping.on_negative.iter() {
                self.events.push_back(Fire(fire_target));
            }
        }
        for (&target, &factor) in self.mouse_wheel_mapping.on_change.iter() {
            self.events.push_back(Value { target, value: value * factor });
        }
    }

    fn set_push_button_targets(&mut self, push_button: PushButton, element_state: ElementState) {
        use self::ElementState::*;
        use self::SwitchState::*;
        use self::ControlEvent::*;

        if let Some(binds) = self.push_button_mapping.get_mut(&push_button) {
            match element_state {
                Pressed => {
                    for &fire_target in binds.on_press.iter() {
                        self.events.push_back(Fire(fire_target));
                    }
                }
                Released => {
                    for &fire_target in binds.on_release.iter() {
                        self.events.push_back(Fire(fire_target));
                    }
                }
            }
            for &switch_target in binds.while_down.iter() {
                let counter = self.switch_counter.get_mut(&switch_target).unwrap();
                if *counter == 0 {
                    self.events.push_back(Switch { target: switch_target, state: Active });
                }
                match element_state {
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
        use self::FireTarget::*;
        use self::SwitchTarget::*;
        use self::ValueTarget::*;
        use self::PushButton::*;
        use self::Bind::*;

        let binds = vec!(
            WhileDown(ScanCode(17), MoveForward),
            WhileDown(ScanCode(31), MoveBackward),
            WhileDown(ScanCode(30), MoveLeft),
            WhileDown(ScanCode(32), MoveRight),
            OnPress(ScanCode(57), Jump),
            OnPress(KeyCode(VirtualKeyCode::Q), Exit),
            OnPress(KeyCode(VirtualKeyCode::Escape), ToggleMenu),
            ForAxis(0, -1.0, Yaw),
            ForAxis(1, -1.0, Pitch),
        );
        Controls::from_binds(binds)
    }
}
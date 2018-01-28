use std;
use std::collections::VecDeque;
use std::collections::HashMap;

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

#[derive(Debug, Copy, Clone)]
pub enum FireTarget {
    Jump,
    Exit,
    ToggleMenu,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum SwitchTarget {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
}

#[derive(Debug, Copy, Clone)]
pub enum ValueTarget {
    Yaw,
    Pitch,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PushButton {
    ScanCode(u32),
    KeyCode(VirtualKeyCode),
    Button(MouseButton),
}

#[derive(Debug)]
enum PushButtonBind {
    OnPress(FireTarget),
    OnRelease(FireTarget),
    WhileDown(SwitchTarget),
}

#[derive(Debug)]
struct ValueBind {
    target: ValueTarget,
    factor: f64,
}

#[derive(Debug)]
enum MouseWheelBind {
    OnPositive(FireTarget),
    OnNegative(FireTarget),
    OnChange(ValueBind),
}

#[derive(Debug)]
pub enum ControlEvent {
    Fire(FireTarget),
    Switch { target: SwitchTarget, state: SwitchState },
    Value { target: ValueTarget, value: f64 },
}

pub struct Controls {
    events: VecDeque<ControlEvent>,
    switch_counter: HashMap<SwitchTarget, u32>,
    push_button_mapping: HashMap<PushButton, PushButtonBind>, // TODO allow multiple binds
    axis_mapping: HashMap<u32, ValueBind>, // TODO allow multiple binds
    mouse_wheel_mapping: Option<MouseWheelBind>, // TODO allow multiple binds
    last_key_state: HashMap<u32, ElementState>,
    last_button_state: HashMap<MouseButton, ElementState>,
}

impl Controls {
    fn new(push_button_mapping: HashMap<PushButton, PushButtonBind>,
           axis_mapping: HashMap<u32, ValueBind>,
           mouse_wheel_mapping: Option<MouseWheelBind>) -> Controls {
        use self::SwitchTarget::*;
        let switch_counter =
            vec!((MoveRight, 0),
                 (MoveLeft, 0),
                 (MoveForward, 0),
                 (MoveBackward, 0),
            ).into_iter().collect(); // TODO find a way to make sure, no target is forgotten
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
        if let Some(&ValueBind { target, factor }) = self.axis_mapping.get(&axis) {
            value *= factor;
            self.events.push_back(Value { target, value });
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
        use self::MouseWheelBind::*;
        use self::ValueBind;
        use self::ControlEvent::*;

        let mut value = match delta {
            LineDelta(_x, y) => y as f64,
            PixelDelta(_x, _y) => return,
        };

        match self.mouse_wheel_mapping {
            Some(OnPositive(fire_target)) => if value > 0.0 {
                self.events.push_back(Fire(fire_target))
            },
            Some(OnNegative(fire_target)) => if value < 0.0 {
                self.events.push_back(Fire(fire_target))
            },
            Some(OnChange(ValueBind { target, factor })) => {
                value *= factor;
                self.events.push_back(Value { target, value });
            },
            None => (),
        }
    }

    fn set_push_button_targets(&mut self, push_button: PushButton, element_state: ElementState) {
        use self::ElementState::*;
        use self::SwitchState::*;
        use self::ControlEvent::*;
        use self::PushButtonBind::*;

        match self.push_button_mapping.get(&push_button) {
            Some(&OnPress(fire_target)) => if element_state == Pressed {
                self.events.push_back(Fire(fire_target));
            },
            Some(&OnRelease(fire_target)) => if element_state == Released {
                self.events.push_back(Fire(fire_target));
            },
            Some(&WhileDown(switch_target)) => {
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
            },
            None => (),
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        use self::FireTarget::*;
        use self::SwitchTarget::*;
        use self::ValueTarget::*;
        use self::PushButton::*;
        use self::PushButtonBind::*;
        use self::MouseWheelBind::*;

        let push_button_mapping = vec!(
            (ScanCode(17), WhileDown(MoveForward)),
            (ScanCode(31), WhileDown(MoveBackward)),
            (ScanCode(30), WhileDown(MoveLeft)),
            (ScanCode(32), WhileDown(MoveRight)),
            (ScanCode(57), OnPress(Jump)),
            (KeyCode(VirtualKeyCode::Q), OnPress(Exit)),
            (KeyCode(VirtualKeyCode::Escape), OnPress(ToggleMenu)),
        ).into_iter().collect();
        let axis_mapping = vec!(
            (0, ValueBind { target: Yaw, factor: -1.0}),
            (1, ValueBind { target: Pitch, factor: -1.0}),
        ).into_iter().collect();
        //let mouse_wheel_mapping = Some(OnChange(ValueBind {target: Pitch, factor: 100.0 }));
        //let mouse_wheel_mapping = Some(OnNegative(ToggleMenu));
        let mouse_wheel_mapping = None;
        Controls::new(push_button_mapping, axis_mapping, mouse_wheel_mapping)
    }
}
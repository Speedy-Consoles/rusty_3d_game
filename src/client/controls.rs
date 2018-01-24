use std::collections::VecDeque;

use super::glium::glutin;

#[derive(Debug)]
pub enum InputState {
    Active,
    Inactive,
}

#[derive(Debug)]
pub enum StateInput {
    MoveRight,
    MoveLeft,
    MoveForward,
    MoveBackward,
}

#[derive(Debug)]
pub enum EventInput {
    Jump,
}

#[derive(Debug)]
pub enum AxisInput {
    Yaw,
    Pitch,
}

#[derive(Debug)]
pub enum InputEvent {
    Trigger(EventInput),
    Move{input: AxisInput, value: f64},
    Toggle {input: StateInput, state: InputState},
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
    // TODO add mapping
}

impl Controls {
    pub fn process_device_event(&mut self, _id: glutin::DeviceId, event: glutin::DeviceEvent) {
        // TODO use variable mapping instead
        use self::glutin::DeviceEvent as DE;
        use self::glutin::ElementState::*;
        use self::InputState::*;
        use self::EventInput::*;
        use self::StateInput::*;
        use self::AxisInput::*;
        use self::InputEvent::*;
        //println!("{:?}", event);
        match event {
            DE::Added => println!("Device added"),
            DE::Removed => println!("Device removed"),
            // MouseMotion unit is sensor(?) unit, not pixels!
            // There will also be a motion event for the axis
            DE::MouseMotion {delta: _d} => (),
            DE::MouseWheel {delta: _d} => (),
            // Motion unit is sensor(?) unit, not pixels!
            DE::Motion {axis: a, value: v} => match a {
                0 => self.input_events.push_back(Move{input: Yaw, value: -v}),
                1 => self.input_events.push_back(Move{input: Pitch, value: -v}),
                _ => (),
            },
            DE::Button {button: _b, state: _s} => (),
            // Key only occurs on state change, no repetition
            DE::Key(ki) => {
                let state = match ki.state {
                    Pressed => Active,
                    Released => Inactive,
                };
                match ki.scancode {
                    17 => self.input_events.push_back(Toggle {input: MoveForward, state: state}),
                    31 => self.input_events.push_back(Toggle {input: MoveBackward, state: state}),
                    30 => self.input_events.push_back(Toggle {input: MoveLeft, state: state}),
                    32 => self.input_events.push_back(Toggle {input: MoveRight, state: state}),
                    57 => self.input_events.push_back(Trigger(Jump)),
                    _  => (),
                }
            },
            DE::Text {codepoint: c} => println!("Text: {}", c),
        }
    }

    pub fn events_iter<'a>(&'a mut self) -> InputEventIterator<'a> {
        InputEventIterator{controls: self}
    }
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            input_events: VecDeque::new(),
        }
    }
}
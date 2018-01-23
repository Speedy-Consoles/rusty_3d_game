use std::collections::VecDeque;

use super::glium::glutin;

#[derive(Debug)]
pub enum InputState {
    On,
    Off,
}

#[derive(Debug)]
pub enum StateInput {
    RotateRight,
    RotateLeft,
}

#[derive(Debug)]
pub enum EventInput {
    Flip,
}

#[derive(Debug)]
pub enum InputEvent {
    Trigger(EventInput),
    Change{input: StateInput, state: InputState},
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
        use self::glutin::DeviceEvent as DE; // WHY self???
        use self::glutin::ElementState::*;
        use self::InputEvent::*;
        use self::EventInput::*;
        use self::StateInput::*;
        use self::InputState::*;
        match event {
            DE::Added => println!("Device added"),
            DE::Removed => println!("Device removed"),
            // MouseMotion unit is sensor(?) unit, not pixels!
            // There will also be a motion event for the axis
            DE::MouseMotion {delta: _d} => (),
            DE::MouseWheel {delta: _d} => (),
            // Motion unit is sensor(?) unit, not pixels!
            DE::Motion {axis: _a, value: _v} => (),
            DE::Button {button: _b, state: _s} => (),
            // Key only occurs on state change, no repetition
            DE::Key(ki) => match ki.scancode {
                30 => if ki.state == Pressed {
                    self.input_events.push_back(Change{input: RotateLeft, state: On});
                } else {
                    self.input_events.push_back(Change{input: RotateLeft, state: Off});
                },
                32 => if ki.state == Pressed {
                    self.input_events.push_back(Change{input: RotateRight, state: On});
                } else {
                    self.input_events.push_back(Change{input: RotateRight, state: Off});
                },
                57 => if ki.state == Pressed {
                    self.input_events.push_back(Trigger(Flip));
                },
                _  => (),
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
use std::collections::VecDeque;
use std::collections::HashSet;

use super::glium::glutin;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum StateCommand {
    RotateRight,
    RotateLeft,
}

#[derive(Debug)]
pub enum EventCommand {
    Flip,
}

pub struct EventCommandIterator<'a> {
    controls: &'a mut Controls,
}

impl<'a> Iterator for EventCommandIterator<'a> {
    type Item = EventCommand;
    fn next(&mut self) -> Option<Self::Item> {
        self.controls.event_commands.pop_front()
    }
}

pub struct Controls {
    event_commands: VecDeque<EventCommand>,
    state_commands: HashSet<StateCommand>, // TODO maybe use something like enum_set instead
    // TODO add mapping
}

impl Controls {
    pub fn process_device_event(&mut self, _id: glutin::DeviceId, event: glutin::DeviceEvent) {
        // TODO use variable mapping instead
        use self::glutin::DeviceEvent as DE; // WHY self???
        use self::glutin::ElementState::*;
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
                    self.state_commands.insert(StateCommand::RotateLeft);
                } else {
                    self.state_commands.remove(&StateCommand::RotateLeft);
                },
                32 => if ki.state == Pressed {
                    self.state_commands.insert(StateCommand::RotateRight);
                } else {
                    self.state_commands.remove(&StateCommand::RotateRight);
                },
                57 => if ki.state == Pressed {
                    self.event_commands.push_back(EventCommand::Flip);
                },
                _  => (),
            },
            DE::Text {codepoint: c} => println!("Text: {}", c),
        }
    }

    pub fn event_commands_iter<'a>(&'a mut self) -> EventCommandIterator<'a> {
        EventCommandIterator{controls: self}
    }

    pub fn state_command_active(&self, command: &StateCommand) -> bool {
        self.state_commands.contains(command)
    }
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            event_commands: VecDeque::new(),
            state_commands: HashSet::new(),
        }
    }
}
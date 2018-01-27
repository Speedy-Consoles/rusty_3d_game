extern crate glium;

mod graphics;
mod controls;
mod server_interface;

use self::glium::glutin;

use consts::TICK_SPEED;
use self::server_interface::ServerInterface;
use self::server_interface::LocalServerInterface;
use model::Model;
use model::world::character::CharacterInput;

pub struct Client {
    events_loop: glutin::EventsLoop,
    graphics: graphics::Graphics,
    controls: controls::Controls,
    closing: bool,
    model: Model,
    server_interface: Box<ServerInterface>,
}

impl Client {
    pub fn new() -> Self {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_fullscreen(events_loop.get_available_monitors().next())
            .with_title("rusty_3d_game");
        let context = glutin::ContextBuilder::new()
            .with_vsync(false);
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display.gl_window().set_cursor(glutin::MouseCursor::NoneCursor);
        display.gl_window().set_cursor_state(glutin::CursorState::Grab).unwrap();

        Client {
            events_loop,
            graphics: graphics::Graphics::new(display),
            controls: Default::default(),
            closing: false,
            model: Model::new(),
            server_interface: Box::new(LocalServerInterface::new()),
        }
    }

    pub fn run(&mut self) {
        use std::time::Duration;
        use std::time::Instant;
        use std::thread;

        // main loop
        let mut next_tick_time = Instant::now();
        while !self.closing {
            next_tick_time += Duration::from_secs(1) / TICK_SPEED;
            self.handle_events();

            let character_input = self.handle_controls();

            self.server_interface.tick(&mut self.model, character_input);

            self.model.tick();

            if next_tick_time > Instant::now() {
                self.graphics.draw(&self.model.get_world());
            }

            let now = Instant::now();
            if next_tick_time > now {
                let sleep_duration = next_tick_time - now;
                thread::sleep(sleep_duration);
            }
        }
    }

    fn handle_events(&mut self) {
        use self::glutin::Event;
        use self::glutin::WindowEvent as WE;

        let mut events = Vec::new();
        self.events_loop.poll_events(|ev| events.push(ev));
        for ev in events {
            match ev {
                // Window events are only received if the window has focus
                Event::WindowEvent {event: wev, ..} => match wev {
                    WE::Resized(width, height) =>
                        self.graphics.set_view_port(width as u64, height as u64),
                    WE::Closed => self.closing = true,
                    WE::DroppedFile(buf) => println!("File dropped: {:?}", buf),
                    WE::HoveredFile(buf) => println!("File hovered: {:?}", buf),
                    WE::HoveredFileCancelled => println!("File hovercanceled"),
                    WE::ReceivedCharacter(_c) => (), // TODO handle chat
                    WE::Focused(_f) => (), // TODO disable controls
                    WE::KeyboardInput {input: i, ..} =>
                        if i.virtual_keycode == Some(glutin::VirtualKeyCode::Q) {
                            self.closing = true
                        }
                    // CursorMoved positions have sub-pixel precision,
                    // but cursor is likely displayed at the rounded-down integer position
                    WE::CursorMoved {position: _p, ..} => (), // TODO handle menu cursor
                    _ => (),
                },
                // Device events are received any time independently of the window focus
                Event::DeviceEvent{event: dev, device_id: id}
                        => self.controls.process_device_event(id, dev),
                Event::Awakened => println!("Event::Awakened"),
                Event::Suspended(sus) => println!("Event::Suspended({})", sus),
            }
        };
    }

    fn handle_controls(&mut self) -> CharacterInput {
        use self::controls::AxisTarget::*;
        use self::controls::StateTarget::*;
        use self::controls::InputEvent::*;
        use self::controls::State::*;

        let mut yaw = self.model.get_world().get_character().get_yaw();
        let mut pitch = self.model.get_world().get_character().get_pitch();
        let mut ci: CharacterInput = CharacterInput::default();
        for ie in self.controls.events_iter() {
            match ie {
                StateEvent {input, state} => {
                    let active = match state { Active => true, Inactive => false};
                    match input {
                        Jump => if active {ci.jumping = true},
                        _ => (),
                    }
                },
                AxisEvent {input, value} => {
                    match input {
                        Yaw => yaw += value / 1000.0,
                        Pitch => pitch += value / 1000.0,
                    }
                },
            }
        }
        ci.set_yaw(yaw);
        ci.set_pitch(pitch);
        ci.forward = match self.controls.get_state(MoveForward) {
            Active => true,
            Inactive => false,
        };
        ci.backward = match self.controls.get_state(MoveBackward) {
            Active => true,
            Inactive => false,
        };
        ci.left = match self.controls.get_state(MoveLeft) {
            Active => true,
            Inactive => false,
        };
        ci.right = match self.controls.get_state(MoveRight) {
            Active => true,
            Inactive => false,
        };
        ci
    }
}

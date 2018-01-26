extern crate glium;

use self::glium::glutin;

use model::Model;

mod graphics;
mod controls;

pub struct Client {
    events_loop: glutin::EventsLoop,
    graphics: graphics::Graphics,
    controls: controls::Controls,
    closing: bool,
    model: Model,
}

impl Client {
    pub fn new() -> Self {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_fullscreen(events_loop.get_available_monitors().next())
            .with_title("rusty_3d_game");
        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display.gl_window().set_cursor(glutin::MouseCursor::NoneCursor);
        display.gl_window().set_cursor_state(glutin::CursorState::Grab).unwrap();

        Client {
            events_loop,
            graphics: graphics::Graphics::new(display),
            controls: Default::default(),
            closing: false,
            model: Model::new(),
        }
    }

    pub fn run(&mut self) {
        // main loop
        while !self.closing {
            self.handle_events();

            {
                use self::controls::EventInput::*;
                use self::controls::StateInput::*;
                use self::controls::AxisInput::*;
                use self::controls::InputEvent::*;
                use self::controls::InputState::*;
                // TODO this should be sent to the server instead
                let mut yaw = self.model.get_world().get_character().get_yaw();
                let mut pitch = self.model.get_world().get_character().get_pitch();
                for ie in self.controls.events_iter() {
                    match ie {
                        Trigger(Jump) => self.model.get_character_input().jump(),
                        Toggle {input: i, state: s} => {
                            let active = match s { Active => true, Inactive => false};
                            match i {
                                MoveRight => self.model.get_character_input().right = active,
                                MoveLeft => self.model.get_character_input().left = active,
                                MoveForward => self.model.get_character_input().forward = active,
                                MoveBackward => self.model.get_character_input().backward = active,
                            }
                        },
                        Move {input: i, value: v} => {
                            match i {
                                Yaw => yaw += v / 1000.0,
                                Pitch => pitch += v / 1000.0,
                            }
                        },
                    }
                }
                self.model.get_character_input().set_yaw(yaw);
                self.model.get_character_input().set_pitch(pitch);
            }
            self.model.tick();
            self.graphics.draw(&self.model.get_world());
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
}

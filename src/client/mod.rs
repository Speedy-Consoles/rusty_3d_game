extern crate glium;

use self::glium::glutin;

use ::world::World;

mod graphics;
mod controls;

pub struct Client {
    events_loop: glutin::EventsLoop,
    graphics: graphics::Graphics,
    controls: controls::Controls,
    closing: bool,
    world: World,
}

impl Client {
    pub fn new() -> Self {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_fullscreen(events_loop.get_available_monitors().next())
            .with_title("rusty_3d_game");
        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display.gl_window().set_cursor(glutin::MouseCursor::Crosshair);

        Client {
            events_loop,
            graphics: graphics::Graphics::new(display),
            controls: Default::default(),
            closing: false,
            world: World::new(),
        }
    }

    pub fn run(&mut self) {
        // main loop
        while !self.closing {
            self.handle_events();

            {
                use self::controls::EventCommand::*;
                use self::controls::StateCommand::*;
                use std::f64::consts::PI;
                // TODO this should be sent to the server instead
                for ec in self.controls.event_commands_iter() {
                    match ec {
                        Flip => self.world.rotate(PI),
                    }
                }
                if self.controls.state_command_active(&RotateRight) {
                    self.world.rotate(-0.1);
                }
                if self.controls.state_command_active(&RotateLeft) {
                    self.world.rotate(0.1);
                }
            }
            self.graphics.draw(&self.world);
        }
    }

    fn handle_events(&mut self) {
        use self::glutin::Event; // WHY self???
        use self::glutin::WindowEvent as WE; // WHY self???

        let mut events = Vec::new();
        self.events_loop.poll_events(|ev| events.push(ev));
        for ev in events {
            match ev {
                // Window events are only received if the window has focus
                Event::WindowEvent {event: wev, ..} => match wev {
                    WE::Resized(_width, _height) => (), // TODO change perspective matrix
                    WE::Closed => self.closing = true,
                    WE::DroppedFile(buf) => println!("File dropped: {:?}", buf),
                    WE::HoveredFile(buf) => println!("File hovered: {:?}", buf),
                    WE::HoveredFileCancelled => println!("File hovercanceled"),
                    WE::ReceivedCharacter(_c) => (), // TODO handle chat
                    WE::Focused(_f) => (), // TODO disable controls
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

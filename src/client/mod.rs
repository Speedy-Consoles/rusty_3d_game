extern crate glium;

use self::glium::glutin;

mod graphics;

pub struct Client {
    events_loop: glutin::EventsLoop,
    graphics: graphics::Graphics,
}

impl Client {
    pub fn new() -> Client {
        let mut events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_fullscreen(events_loop.get_available_monitors().next())
            .with_title("rusty_3d_game");
        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display.gl_window().set_cursor(glutin::MouseCursor::Crosshair);

        Client {
            events_loop,
            graphics: graphics::Graphics::new(display),
        }
    }

    pub fn run(&mut self) {
        // main loop
        let mut closed = false;
        while !closed {
            self.handle_events();
            self.graphics.draw();
        }
    }

    fn handle_events(&mut self) {
        use std;

        use self::glutin::Event; // WHY self???
        use self::glutin::WindowEvent as WE; // WHY self???
        use self::glutin::DeviceEvent as DE; // WHY self???

        let mut events = Vec::new();
        self.events_loop.poll_events(|ev| events.push(ev));
        for ev in events {
            match ev {
                // Window events are only received if the window has focus
                Event::WindowEvent {event: wev, ..} => match wev {
                    WE::Resized(width, size) => (),
                    WE::Moved(delta_x, delta_y) => (),
                    WE::Closed => std::process::exit(0),
                    WE::DroppedFile(buf) => println!("File dropped: {:?}", buf),
                    WE::HoveredFile(buf) => println!("File hovered: {:?}", buf),
                    WE::HoveredFileCancelled => println!("File hovercanceled"),
                    // ReceivedCharacter repetively triggers when you hold the key
                    WE::ReceivedCharacter(c) => (),
                    WE::Focused(f) => (),
                    // KeyboardInput repetively triggers when you hold the key
                    WE::KeyboardInput {..} => (),
                    // CursorMoved positions have sub-pixel precision,
                    // but cursor is likely displayed at the rounded-down integer position
                    WE::CursorMoved {position: p, ..} => (),
                    WE::CursorEntered {..} => (),
                    WE::CursorLeft {..} => (),
                    WE::MouseWheel {device_id: id, delta, phase} => (),
                    WE::MouseInput {device_id: id, state, button} => (),
                    WE::TouchpadPressure {..} => (),
                    // AxisMotion: for mouse: absolute position on display
                    WE::AxisMotion {device_id: id, axis, value} => (),
                    WE::Refresh => (), // TODO
                    WE::Touch(_) => (),
                    WE::HiDPIFactorChanged(_) => (),
                },
                // Device events are received any time independently of the window focus
                Event::DeviceEvent {event: dev, ..} => match dev {
                    DE::Added => println!("Device added"),
                    DE::Removed => println!("Device removed"),
                    // MouseMotion unit is sensor(?) unit, not pixels!
                    // There will also be a motion event for the axis
                    DE::MouseMotion {delta} => (),
                    DE::MouseWheel {delta} => (),
                    // Motion unit is sensor(?) unit, not pixels!
                    DE::Motion {axis: a, value: v} => (),
                    DE::Button {button, state} => (),
                    // Key only occurs on state change, no repetition
                    DE::Key(input) => (),
                    DE::Text {codepoint: c} => println!("Text: {}", c),
                }
                Event::Awakened => println!("Event::Awakened"),
                Event::Suspended(sus) => println!("Event::Suspended({})", sus),
                _ => println!("DeviceEvent::{:?}", ev),
            }
        };
    }

    fn handle_input(&mut self, event: glutin::WindowEvent) {
        // TODO
    }
}

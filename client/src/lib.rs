mod graphics;
mod controls;
mod config;
mod server_interface;

#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate toml;
extern crate num;
extern crate strum;
#[macro_use]
extern crate strum_macros;

extern crate shared;

use std::time::Instant;
use std::thread;

use glium::glutin;
use glium::backend::glutin::Display;

use shared::model::Model;
use shared::model::world::character::CharacterInput;
use server_interface::ServerInterface;
use server_interface::LocalServerInterface;
use config::Config;

pub struct Client {
    events_loop: glutin::EventsLoop,
    server_interface: Box<ServerInterface>,
    graphics: graphics::Graphics,
    display: Display,
    config: Config,
    model: Model,
    closing: bool,
    menu_active: bool,
    cursor_grabbed: bool,
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

        let config = match Config::load() {
            Ok(c) => c,
            Err(err) => {
                println!("Error while loading config: {}", err);
                let c = Config::default();
                if let Err(err) = c.save() {
                    println!("Error while saving config: {}", err);
                }
                c
            }
        };

        Client {
            events_loop,
            server_interface: Box::new(LocalServerInterface::new()),
            graphics: graphics::Graphics::new(&display),
            display,
            config,
            model: Model::new(),
            closing: false,
            menu_active: true,
            cursor_grabbed: false,
        }
    }

    pub fn run(&mut self) {
        // main loop
        while !self.closing {
            self.handle_events();

            let character_input = self.handle_controls();

            self.server_interface.tick(&mut self.model, character_input);
            let next_tick_time = self.server_interface.get_next_tick_time();

            self.model.tick();

            if self.menu_active == self.cursor_grabbed {
                let menu_active = self.menu_active;
                self.try_set_cursor_grab(!menu_active);
            }
            if next_tick_time > Instant::now() {
                self.graphics.draw(&self.model.get_world(), &self.display);
            }

            let now = Instant::now();
            if next_tick_time > now {
                let sleep_duration = next_tick_time - now;
                thread::sleep(sleep_duration);
            }
        }

        // clean up grab, because it might cause errors otherwise
        self.try_set_cursor_grab(false);
    }

    fn set_menu(&mut self, active: bool) {
        if active == self.menu_active {
            return;
        }
        self.menu_active = active;
        if self.menu_active {
            self.display.gl_window().set_cursor(glutin::MouseCursor::Default);
        } else {
            self.display.gl_window().set_cursor(glutin::MouseCursor::NoneCursor);
        }
    }

    fn try_set_cursor_grab(&mut self, grab: bool) {
        if grab {
            self.cursor_grabbed
                = self.display.gl_window().set_cursor_state(glutin::CursorState::Grab).is_ok();
        } else {
            self.cursor_grabbed
                = !self.display.gl_window().set_cursor_state(glutin::CursorState::Normal).is_ok();
        }
    }

    fn handle_events(&mut self) {
        use self::glutin::Event::*;
        use self::glutin::WindowEvent as WE;
        use self::glutin::DeviceEvent as DE;

        let mut events = Vec::new(); // TODO get rid of allocation
        self.events_loop.poll_events(|ev| events.push(ev));
        for ev in events {
            match ev {
                // Window events are only received if the window has focus
                WindowEvent { event: wev, .. } => match wev {
                    WE::Resized(width, height) =>
                        self.graphics.set_view_port(width as u64, height as u64),
                    WE::Closed => self.closing = true,
                    WE::DroppedFile(buf) => println!("File dropped: {:?}", buf),
                    WE::HoveredFile(buf) => println!("File hovered: {:?}", buf),
                    WE::HoveredFileCancelled => println!("File hover canceled"),
                    WE::ReceivedCharacter(_c) => (), // TODO handle chat
                    WE::Focused(false) => self.set_menu(true),
                    WE::KeyboardInput { device_id, input } =>
                        self.config.controls.process_keyboard_input_event(device_id, input),
                    WE::MouseInput { device_id, state, button, modifiers } =>
                        self.config.controls.process_mouse_input_event(device_id, state,
                                                                button, modifiers),
                    WE::MouseWheel {device_id, delta, phase, modifiers} =>
                        self.config.controls
                            .process_mouse_wheel_event(device_id, delta, phase, modifiers),
                    // CursorMoved positions have sub-pixel precision,
                    // but cursor is likely displayed at the rounded-down integer position
                    WE::CursorMoved {position: _p, ..} => (), // TODO handle menu cursor
                    _ => (),
                },
                // Device events are received any time independently of the window focus
                DeviceEvent { device_id, event } =>
                    if let DE::Motion { axis, value } = event {
                        self.config.controls.process_motion_event(device_id, axis, value);
                    },
                Awakened => println!("Event::Awakened"),
                Suspended(sus) => println!("Event::Suspended({})", sus),
            }
        };
    }

    fn handle_controls(&mut self) -> CharacterInput {
        use controls::FireTarget::*;
        use controls::SwitchTarget::*;
        use controls::ValueTarget::*;
        use controls::ControlEvent::*;
        use controls::SwitchState::*;

        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        let mut jumping = false;
        for ie in self.config.controls.get_events() {
            match ie {
                Fire(target) => {
                    match target {
                        Jump => jumping = true,
                        NextWeapon => println!("next weapon"),
                        PrevWeapon => println!("previous weapon"),
                        ToggleMenu => {
                            let menu_active = self.menu_active;
                            self.set_menu(!menu_active);
                        },
                        Exit => self.closing = true,
                    }
                },
                Value {target: Yaw, value} => yaw_delta += value / 1000.0,
                Value {target: Pitch, value} => pitch_delta += value / 1000.0,
                Switch { target: Shoot, state: Active } => println!("pew"),
                Switch { target: Aim, state: Active } => println!("aim"),
                _ => (),
            }
        }
        // TODO maybe we shouldn't take these values from the shared.model
        let old_yaw = self.model.get_world().get_character().get_yaw();
        let old_pitch = self.model.get_world().get_character().get_pitch();
        let mut ci = CharacterInput::default();
        if !self.menu_active {
            ci.set_yaw(old_yaw + yaw_delta);
            ci.set_pitch(old_pitch + pitch_delta);
            ci.jumping = jumping;
            ci.forward = match self.config.controls.get_state(MoveForward) {
                Active => true,
                Inactive => false,
            };
            ci.backward = match self.config.controls.get_state(MoveBackward) {
                Active => true,
                Inactive => false,
            };
            ci.left = match self.config.controls.get_state(MoveLeft) {
                Active => true,
                Inactive => false,
            };
            ci.right = match self.config.controls.get_state(MoveRight) {
                Active => true,
                Inactive => false,
            };
        } else {
            ci.set_yaw(old_yaw);
            ci.set_pitch(old_pitch);
        }
        ci
    }
}

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

use shared::consts;
use shared::consts::DRAW_SPEED;
use shared::util;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use graphics::Graphics;
use server_interface::ServerInterface;
use server_interface::LocalServerInterface;
use config::Config;

pub struct Client {
    events_loop: glutin::EventsLoop,
    server_interface: Box<ServerInterface>,
    graphics: Graphics,
    display: Display,
    config: Config,
    model: Model,
    character_input: CharacterInput,
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
            graphics: Graphics::new(&display),
            display,
            config,
            model: Model::new(),
            character_input: Default::default(),
            closing: false,
            menu_active: true,
            cursor_grabbed: false,
        }
    }

    pub fn run(&mut self) {
        // for fps display
        let mut last_sec = Instant::now();
        let mut tick_counter = 0;
        let mut draw_counter = 0;

        // for sleep timing
        let mut next_draw_time = Instant::now();
        let mut next_tick_time = Instant::now();

        // main loop
        while !self.closing {
            // events
            self.handle_events();
            self.handle_controls();

            // tick
            let now = Instant::now();
            if now >= next_tick_time {
                let mut character_input = self.character_input;
                if self.menu_active {
                    character_input = Default::default();
                    character_input.add_yaw(self.character_input.get_yaw());
                    character_input.add_pitch(self.character_input.get_pitch());
                }
                self.server_interface.tick(&mut self.model, character_input);
                self.character_input.reset_flags();
                next_tick_time = self.server_interface.get_next_tick_time();
                tick_counter += 1;
            }

            if self.menu_active == self.cursor_grabbed {
                let menu_active = self.menu_active;
                self.try_set_cursor_grab(!menu_active);
            }

            // draw
            let now = Instant::now();
            if now >= next_draw_time {
                self.graphics.draw(
                    &self.model.get_world(),
                    self.server_interface.get_tick(),
                    self.server_interface.get_intra_tick(),
                    &self.display
                );
                let now = Instant::now();
                let diff = now - next_draw_time;
                let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 1e-9;
                let whole_draw_diff = (sec_diff * DRAW_SPEED as f64).floor() as u64;
                next_draw_time +=
                    util::mult_duration(&consts::draw_interval(), whole_draw_diff + 1);
                draw_counter += 1;
            }

            // display rates
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}, draws/s: {}", tick_counter, draw_counter);
                tick_counter = 0;
                draw_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
            }

            // sleep
            let next_loop_time = next_tick_time.min(next_draw_time);
            let now = Instant::now();
            if next_loop_time > now {
                let sleep_duration = next_loop_time - now;
                thread::sleep(sleep_duration); // TODO handle network
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

    fn handle_controls(&mut self) {
        use controls::FireTarget::*;
        use controls::SwitchTarget::*;
        use controls::ValueTarget::*;
        use controls::ControlEvent::*;
        use controls::SwitchState::*;

        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        for ie in self.config.controls.get_events() {
            match ie {
                Fire(target) => {
                    match target {
                        Jump => self.character_input.jumping = true,
                        NextWeapon => println!("next weapon"),
                        PrevWeapon => println!("previous weapon"),
                        ToggleMenu => {
                            let menu_active = self.menu_active;
                            self.set_menu(!menu_active);
                        },
                        Exit => self.closing = true,
                    }
                },
                Value { target: Yaw, value } => yaw_delta += value,
                Value { target: Pitch, value } => pitch_delta += value,
                Switch { target, state} => match target {
                    Shoot => if state == Active { println!("pew") },
                    Aim => if state == Active { println!("aim") },
                    MoveForward => self.character_input.forward = state == Active,
                    MoveBackward => self.character_input.backward = state == Active,
                    MoveLeft => self.character_input.left = state == Active,
                    MoveRight => self.character_input.right = state == Active,
                }
            }
        }
        if !self.menu_active {
            self.character_input.add_yaw(yaw_delta);
            self.character_input.add_pitch(pitch_delta);
        }
    }
}

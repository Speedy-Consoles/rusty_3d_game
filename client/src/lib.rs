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
use std::env;
use std::net::ToSocketAddrs;

use glium::glutin;
use glium::backend::glutin::Display;

use shared::math::FPAngle;
use shared::consts;
use shared::consts::DRAW_SPEED;
use shared::util;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::CharacterInput;
use graphics::Graphics;
use server_interface::ServerInterface;
use server_interface::LocalServerInterface;
use server_interface::RemoteServerInterface;
use config::Config;

pub struct Client {
    events_loop: glutin::EventsLoop,
    server_interface: Box<ServerInterface>,
    graphics: Graphics,
    display: Display,
    config: Config,
    model: Model,
    predicted_world: World,
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

        let si: Box<ServerInterface> = match env::args().nth(1) {
            Some(addr_string) => {
                let mut addrs = addr_string.to_socket_addrs().unwrap();
                Box::new(RemoteServerInterface::new(addrs.next().unwrap()).unwrap())
            },
            None => Box::new(LocalServerInterface::new()),
        };

        Client {
            events_loop,
            server_interface: si,
            graphics: Graphics::new(&display),
            display,
            config,
            model: Model::new(),
            predicted_world: World::new(),
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
            let before_tick = Instant::now();
            if before_tick >= next_tick_time {
                let mut character_input = self.character_input;
                if self.menu_active {
                    character_input = Default::default();
                    character_input.view_dir = self.character_input.view_dir;
                }
                self.server_interface.tick(&mut self.model, character_input);
                self.character_input.reset_flags();
                if let Some(tick_info) = self.server_interface.get_tick_info() {
                    let tick_lag = self.server_interface.get_tick_lag();
                    self.predict(tick_info.tick + 1, tick_lag);
                    next_tick_time = tick_info.tick_time + consts::tick_interval();
                } else {
                    next_tick_time = before_tick + consts::tick_interval();
                }
                tick_counter += 1;
            }

            if self.menu_active == self.cursor_grabbed {
                let menu_active = self.menu_active;
                self.try_set_cursor_grab(!menu_active);
            }

            // draw
            let before_draw = Instant::now();
            if before_draw >= next_draw_time {
                if let Some(my_player_id) = self.server_interface.get_my_player_id() {
                    if let Some(tick_info) = self.server_interface.get_tick_info() {
                        let view_dir = if self.config.direct_camera {
                            Some(self.character_input.view_dir)
                        } else {
                            None
                        };
                        self.graphics.draw(
                            &self.model,
                            &self.predicted_world,
                            my_player_id,
                            view_dir,
                            tick_info.tick,
                            tick_info.get_intra_tick(),
                            &self.display
                        );
                        draw_counter += 1;
                    }
                }
                let draw_diff = util::elapsed_ticks(next_draw_time.elapsed(), DRAW_SPEED);
                next_draw_time += util::mult_duration(consts::draw_interval(), draw_diff + 1);
            }

            // display rates
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}, draws/s: {}", tick_counter, draw_counter);
                tick_counter = 0;
                draw_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
            }

            // sleep / handle traffic
            self.server_interface.handle_traffic(next_tick_time.min(next_draw_time));
        }

        self.server_interface.disconnect();

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

    fn predict(&mut self, start_tick: u64, num_ticks: u64) {
        if let Some(my_id) = self.server_interface.get_my_player_id() {
            self.predicted_world = self.model.get_world().clone();
            for tick in start_tick..(start_tick + num_ticks) {
                if let Some(input) = self.server_interface.get_character_input(tick) {
                    self.predicted_world.set_character_input(my_id, input);
                }
                self.predicted_world.tick();
            }
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
                    Crouch => self.character_input.crouch = state == Active,
                }
            }
        }
        if !self.menu_active {
            self.character_input.view_dir.add_yaw(FPAngle::from_tau_float(yaw_delta));
            self.character_input.view_dir.add_pitch(FPAngle::from_tau_float(pitch_delta));
        }
    }
}

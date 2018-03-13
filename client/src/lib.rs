mod graphics;
mod controls;
mod config;
mod server_interface;
mod menu;

#[macro_use] extern crate glium;
extern crate cgmath;
extern crate toml;
extern crate num;
extern crate strum;
#[macro_use] extern crate strum_macros;

extern crate shared;

use std::time::Instant;
use std::env;
use std::net::ToSocketAddrs;

use glium::glutin;
use glium::backend::glutin::Display;

use shared::math::FPAngle;
use shared::consts::TICK_SPEED;
use shared::consts::DRAW_SPEED;
use shared::model::world::character::CharacterInput;

use graphics::Graphics;
use server_interface::ServerInterface;
use server_interface::LocalServerInterface;
use server_interface::RemoteServerInterface;
use server_interface::ConnectionState::*;
use config::Config;
use menu::Menu;

pub struct Client {
    events_loop: glutin::EventsLoop,
    server_interface: Box<ServerInterface>,
    graphics: Graphics,
    display: Display,
    config: Config,
    character_input: CharacterInput,
    closing: bool,
    menu: Menu,
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
            character_input: Default::default(),
            closing: false,
            menu: Menu::new(),
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
        loop {
            // events
            self.handle_events();
            self.handle_controls();

            // tick
            let before_tick = Instant::now();
            if before_tick >= next_tick_time {
                let mut character_input = self.character_input;
                if self.menu.active() {
                    character_input = Default::default();
                    character_input.view_dir = self.character_input.view_dir;
                }
                self.server_interface.do_tick(character_input);
                self.character_input.reset_flags();
                if let Connected { tick_info, .. } = self.server_interface.connection_state() {
                    next_tick_time = tick_info.next_tick_time;
                } else {
                    next_tick_time = next_tick_time + 1 / TICK_SPEED;
                }
                tick_counter += 1;
            }

            self.update_cursor();

            // draw
            let before_draw = Instant::now();
            if before_draw >= next_draw_time {
                if let Connected { tick_info, my_player_id, model, predicted_world }
                        = self.server_interface.connection_state() {
                    let view_dir = if self.config.direct_camera {
                        Some(self.character_input.view_dir)
                    } else {
                        None
                    };

                    self.graphics.draw(
                        model,
                        predicted_world,
                        my_player_id,
                        view_dir,
                        tick_info.now(),
                        &self.display
                    );
                    draw_counter += 1;
                }
                let draw_tick_diff = (next_draw_time.elapsed() * DRAW_SPEED).ticks;
                next_draw_time += (draw_tick_diff + 1) / DRAW_SPEED;
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

            if self.closing {
                self.server_interface.disconnect();
                if let Disconnected = self.server_interface.connection_state() {
                    break;
                }
            }
        }
    }

    fn update_cursor(&mut self) {
        let window = self.display.gl_window();
        if self.menu.active() {
            window.set_cursor(glutin::MouseCursor::Default);
            window.set_cursor_state(glutin::CursorState::Normal).is_ok();
        } else {
            window.set_cursor(glutin::MouseCursor::NoneCursor);
            window.set_cursor_state(glutin::CursorState::Grab).is_ok();
        }
    }

    fn handle_events(&mut self) {
        use self::glutin::Event::*;
        use self::glutin::WindowEvent as WE;
        use self::glutin::DeviceEvent as DE;

        let graphics = &mut self.graphics;
        let closing = &mut self.closing;
        let menu = &mut self.menu;
        let config = &mut self.config;
        self.events_loop.poll_events(|ev| {
            match ev {
                // Window events are only received if the window has focus
                WindowEvent { event: wev, .. } => match wev {
                    WE::Resized(width, height) =>
                        graphics.set_view_port(width as u64, height as u64),
                    WE::Closed => *closing = true,
                    WE::DroppedFile(buf) => println!("File dropped: {:?}", buf),
                    WE::HoveredFile(buf) => println!("File hovered: {:?}", buf),
                    WE::HoveredFileCancelled => println!("File hover canceled"),
                    WE::ReceivedCharacter(_c) => (), // TODO handle chat
                    WE::Focused(false) => menu.set_active(true),
                    WE::KeyboardInput { device_id, input } =>
                        config.controls.process_keyboard_input_event(device_id, input),
                    WE::MouseInput { device_id, state, button, modifiers } =>
                        config.controls.process_mouse_input_event(device_id, state,
                                                                button, modifiers),
                    WE::MouseWheel {device_id, delta, phase, modifiers} =>
                        config.controls
                            .process_mouse_wheel_event(device_id, delta, phase, modifiers),
                    // CursorMoved positions have sub-pixel precision,
                    // but cursor is likely displayed at the rounded-down integer position
                    WE::CursorMoved {position: _p, ..} => (), // TODO handle menu cursor
                    _ => (),
                },
                // Device events are received any time independently of the window focus
                DeviceEvent { device_id, event } =>
                    if let DE::Motion { axis, value } = event {
                        config.controls.process_motion_event(device_id, axis, value);
                    },
                Awakened => println!("Event::Awakened"),
                Suspended(sus) => println!("Event::Suspended({})", sus),
            }
        });
    }

    fn handle_controls(&mut self) {
        use controls::FireTarget::*;
        use controls::SwitchTarget::*;
        use controls::ValueTarget::*;
        use controls::ControlEvent::*;
        use controls::SwitchState::*;

        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        for ie in self.config.controls.events() {
            match ie {
                Fire(target) => {
                    match target {
                        Jump => self.character_input.jumping = true,
                        NextWeapon => println!("next weapon"),
                        PrevWeapon => println!("previous weapon"),
                        ToggleMenu => {
                            let menu_active = self.menu.active();
                            self.menu.set_active(!menu_active);
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
        if !self.menu.active() {
            self.character_input.view_dir.add_yaw(FPAngle::from_tau_float(yaw_delta));
            self.character_input.view_dir.add_pitch(FPAngle::from_tau_float(pitch_delta));
        }
    }
}

mod graphics;
mod controls;
mod config;
mod server_interface;
mod menu;

#[macro_use] extern crate glium;
extern crate glium_text;
extern crate cgmath;
extern crate toml;
extern crate num;
extern crate strum;
#[macro_use] extern crate strum_macros;
extern crate rand;
extern crate arrayvec;

extern crate shared;

use std::time::Instant;
use std::env;
use std::net::ToSocketAddrs;

use glium::glutin;
use glium::backend::glutin::Display;

use shared::math::FPAngle;
use shared::consts::BASE_SPEED;
use shared::consts::DRAW_SPEED;
use shared::model::world::character::CharacterInput;

use graphics::Graphics;
use server_interface::ServerInterface;
use server_interface::LocalServerInterface;
use server_interface::RemoteServerInterface;
use server_interface::ConnectionState::*;
use server_interface::HandleTrafficResult;
use config::Config;
use menu::Menu;
use TickTarget::*;

enum TickTarget {
    GameTick,
    SocketTick,
    GraphicsTick,
    BaseTick,
}

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
        let mut next_draw_time = Some(Instant::now());

        // main loop
        loop {
            // window events, controls
            self.handle_events();
            self.handle_controls();

            // sleep / handle traffic
            let mut tick_target = BaseTick;
            let mut next_loop_time = Instant::now() + 1 / BASE_SPEED;
            loop {
                match self.server_interface.next_game_tick_time() {
                    Some(next_game_tick_time) if next_game_tick_time < next_loop_time => {
                        tick_target = GameTick;
                        next_loop_time = next_game_tick_time;
                    },
                    _ => (),
                }
                match self.server_interface.next_socket_tick_time() {
                    Some(next_socket_tick_time) if next_socket_tick_time < next_loop_time => {
                        tick_target = SocketTick;
                        next_loop_time = next_socket_tick_time;
                    },
                    _ => (),
                }
                match next_draw_time {
                    Some(next_graphics_tick_time) if next_graphics_tick_time < next_loop_time => {
                        tick_target = GraphicsTick;
                        next_loop_time = next_graphics_tick_time;
                    },
                    _ => (),
                }
                if let HandleTrafficResult::Timeout
                        = self.server_interface.handle_traffic(next_loop_time) {
                    break;
                };
                // TODO maybe add conditional break here, to make sure the client stays responsive on DDoS
            }

            // handle closing request
            if self.closing {
                // wait for disconnect before closing
                match self.server_interface.connection_state() {
                    Connecting | Connected { .. } => self.server_interface.disconnect(),
                    Disconnecting => (),
                    Disconnected(_) => break,
                }
            }

            // tick
            match tick_target {
                GameTick => {
                    let mut character_input = self.character_input;
                    self.server_interface.do_tick(character_input);
                    tick_counter += 1;
                },
                SocketTick => self.server_interface.do_socket_tick(),
                GraphicsTick => {
                    if let Connected {
                        tick_instant,
                        my_player_id,
                        model,
                        predicted_world,
                    } = self.server_interface.connection_state() {
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
                            tick_instant,
                            &self.display
                        );
                        draw_counter += 1;
                    }
                    if let Some(mut next_graphics_tick) = next_draw_time {
                        let now = Instant::now();
                        let draw_tick_diff = if now < next_graphics_tick {
                            0
                        } else {
                            ((now - next_graphics_tick) * DRAW_SPEED).ticks
                        };
                        next_graphics_tick += (draw_tick_diff + 1) / DRAW_SPEED;
                        next_draw_time = Some(next_graphics_tick);
                    }
                },
                BaseTick => (),
            }

            self.update_cursor();

            // display rates
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}, draws/s: {}", tick_counter, draw_counter);
                tick_counter = 0;
                draw_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
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

        let mut character_input = self.character_input;
        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        for ie in self.config.controls.events() {
            match ie {
                Fire(target) => {
                    match target {
                        Jump => character_input.num_jumps += 1,
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
                    MoveForward => character_input.forward = state == Active,
                    MoveBackward => character_input.backward = state == Active,
                    MoveLeft => character_input.left = state == Active,
                    MoveRight => character_input.right = state == Active,
                    Crouch => character_input.crouch = state == Active,
                }
            }
        }
        character_input.view_dir.add_yaw(FPAngle::from_tau_float(yaw_delta));
        character_input.view_dir.add_pitch(FPAngle::from_tau_float(pitch_delta));
        if !self.menu.active() {
            self.character_input = character_input;
        }
    }
}

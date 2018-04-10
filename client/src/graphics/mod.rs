mod model_graphics;

use std::fs::File;

use glium::Display;
use glium::Frame;
use glium::Surface;
use glium_text;
use glium_text::TextSystem;
use glium_text::FontTexture;
use glium_text::TextDisplay;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::SquareMatrix;

use shared::consts::OPTIMAL_SCREEN_RATIO;
use shared::consts::DEBUG_TEXT_FONT_SIZE;
use shared::consts::DEBUG_TEXT_HEIGHT;
use shared::consts::DEBUG_TEXT_RELATIVE_LINE_HEIGHT;
use shared::model::world::character::ViewDir;

use server_interface::ConnectionState;

use self::model_graphics::ModelGraphics;

pub struct Graphics {
    model_graphics: ModelGraphics,
    text_system: TextSystem,
    font: FontTexture,
    debug_text_matrix: Matrix4<f32>,
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        let font_file = File::open("SourceCodeVariable-Roman.ttf").expect("Could not load font!");
        let font = FontTexture::new(display, &font_file, DEBUG_TEXT_FONT_SIZE).unwrap();

        Graphics {
            model_graphics: ModelGraphics::new(display),
            text_system: TextSystem::new(display),
            font,
            debug_text_matrix: Matrix4::identity().into(),
        }
    }

    pub fn draw(&mut self, connection_state: ConnectionState, view_dir: Option<ViewDir>,
                display: &Display) {
        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_depth(1.0);

        match connection_state {
            ConnectionState::Connecting => (), // TODO
            ConnectionState::Connected {
                tick_instant,
                my_player_id,
                model,
                predicted_world,
            } => {
                self.model_graphics.draw(
                    model,
                    predicted_world,
                    my_player_id,
                    view_dir,
                    tick_instant,
                    &mut frame,
                );
            },
            ConnectionState::Disconnecting => (), // TODO
            ConnectionState::Disconnected(_) => (), // TODO
        }

        self.draw_debug_info(&mut frame, connection_state);

        frame.finish().unwrap();
    }

    fn draw_debug_info(&self, frame: &mut Frame, connection_state: ConnectionState) {
        let tick_text;
        let connection_state_text;
        match connection_state {
            ConnectionState::Connected { tick_instant, .. } => {
                tick_text = format!("{}", tick_instant.tick);
                connection_state_text = String::from("connected"); // TODO no allocation
            },
            _ => {
                tick_text = String::from("---"); // TODO no allocation
                connection_state_text = String::from("---"); // TODO no allocation
            },
        }

        let debug_text = format!(
            "\
                Connection state: {}\n\
                Tick: {}\n\
            ",
            connection_state_text,
            tick_text,
        );

        for (i, line) in debug_text.lines().enumerate() {
            self.draw_debug_line(frame, i as u64, line);
        }
    }

    fn draw_debug_line(&self, frame: &mut Frame, line_number: u64, text: &str) {
        // TODO does this have (gpu) allocations?
        let text_display = TextDisplay::new(&self.text_system, &self.font, text);

        let y_offset = -DEBUG_TEXT_RELATIVE_LINE_HEIGHT * (line_number + 1) as f64;
        let x_offset = DEBUG_TEXT_RELATIVE_LINE_HEIGHT - 1.0;
        let translation = Vector3::new(x_offset as f32, y_offset as f32, 0.0);

        let trafo_matrix = Matrix4::from_translation(translation);
        let matrix: [[f32; 4]; 4] = (self.debug_text_matrix * trafo_matrix).into();

        glium_text::draw(
            &text_display,
            &self.text_system,
            frame,
            matrix,
            (1.0, 1.0, 1.0, 1.0),
        );
    }

    pub fn set_view_port(&mut self, width: u64, height: u64) {
        let ratio = if height != 0 {
            width as f64 / height as f64
        } else {
            0.0
        };
        self.model_graphics.set_screen_ratio(ratio);
        self.build_debug_text_matrix(ratio);
    }

    fn build_debug_text_matrix(&mut self, screen_ratio: f64) {
        let scaling_factor;
        let mut x_offset = 0.0;
        let mut y_offset = 0.0;
        if screen_ratio > OPTIMAL_SCREEN_RATIO {
            scaling_factor = OPTIMAL_SCREEN_RATIO / screen_ratio;
            x_offset = 1.0 - scaling_factor as f32;
        } else {
            scaling_factor = screen_ratio / OPTIMAL_SCREEN_RATIO;
            y_offset = 1.0 - scaling_factor as f32;
        }
        let y_scaling = (DEBUG_TEXT_HEIGHT * scaling_factor) as f32;
        let x_scaling = (DEBUG_TEXT_HEIGHT * scaling_factor / screen_ratio) as f32;
        self.debug_text_matrix = Matrix4::new(
             x_scaling,      0.0,            0.0, 0.0,
             0.0,            y_scaling,      0.0, 0.0,
             0.0,            0.0,            1.0, 0.0,
            -1.0 + x_offset, 1.0 - y_offset, 0.0, 1.0f32,
        );
    }
}
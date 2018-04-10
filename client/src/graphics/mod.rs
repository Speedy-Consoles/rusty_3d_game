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
use cgmath::SquareMatrix;

use shared::consts::OPTIMAL_SCREEN_RATIO;
use shared::model::world::character::ViewDir;

use server_interface::ConnectionState;

use self::model_graphics::ModelGraphics;

pub struct Graphics {
    model_graphics: ModelGraphics,
    text_system: TextSystem,
    font: FontTexture,
    debug_text_matrix: [[f32; 4]; 4],
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        let font_file = File::open("SourceCodeVariable-Roman.ttf").expect("Could not load font!");
        let font = FontTexture::new(display, &font_file, 35).unwrap();

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
        // TODO fewer allocations
        let tick_text = format!(
            "Tick: {}",
            match connection_state {
                ConnectionState::Connected { tick_instant, .. } => format!("{}", tick_instant.tick),
                _ => String::from("---"),
            }
        );
        let text_display = TextDisplay::new(&self.text_system, &self.font, &tick_text);
        let text_width = text_display.get_width();

        let (w, h) = frame.get_dimensions();

        glium_text::draw(
            &text_display,
            &self.text_system,
            frame,
            self.debug_text_matrix,
            (1.0, 1.0, 1.0, 1.0)
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

    fn build_debug_text_matrix(&mut self, mut screen_ratio: f64) {
        let mut font_scaling = 0.04;
        let mut scaling_factor = 1.0;
        let mut x_offset = 0.0;
        let mut y_offset = 0.0;
        if screen_ratio > OPTIMAL_SCREEN_RATIO {
            scaling_factor = OPTIMAL_SCREEN_RATIO / screen_ratio;
            x_offset = 1.0 - scaling_factor as f32;
        } else {
            scaling_factor = screen_ratio / OPTIMAL_SCREEN_RATIO;
            y_offset = 1.0 - scaling_factor as f32;
        }
        let y_scaling = (font_scaling * scaling_factor) as f32;
        let x_scaling = (font_scaling * scaling_factor / screen_ratio) as f32;
        self.debug_text_matrix = Matrix4::new(
            x_scaling,        0.0,              0.0, 0.0,
            0.0,              y_scaling,        0.0, 0.0,
            0.0,              0.0,              1.0, 0.0,
            -1.0 + x_offset,  -1.0 + y_offset,   0.0, 1.0f32,
        ).into();
    }
}
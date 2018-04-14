mod model_graphics;
mod debug_graphics;

use glium::Display;
use glium::Surface;

use shared::model::world::character::ViewDir;

use server_interface::ConnectionState;

use self::model_graphics::ModelGraphics;
use self::debug_graphics::DebugGraphics;

pub struct Graphics {
    model_graphics: ModelGraphics,
    debug_graphics: DebugGraphics,
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        Graphics {
            model_graphics: ModelGraphics::new(display),
            debug_graphics: DebugGraphics::new(display),
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

        self.debug_graphics.draw(&mut frame, connection_state);

        frame.finish().unwrap();
    }

    pub fn set_view_port(&mut self, width: u64, height: u64) {
        let ratio = if height != 0 {
            width as f64 / height as f64
        } else {
            0.0
        };
        self.model_graphics.set_screen_ratio(ratio);
        self.debug_graphics.set_screen_ratio(ratio);
    }
}
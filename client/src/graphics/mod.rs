mod visual_world;

use std;
use std::io::Read;
use std::f64::consts::PI;
use std::mem;
use std::fs::File;
use std::io::BufReader;

use glium;
use glium::Display;
use glium::Surface;
use glium::Frame;
use glium::draw_parameters::Depth;
use glium::draw_parameters::DrawParameters;
use glium::vertex::EmptyVertexAttributes;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium_text;
use glium_text::TextSystem;
use glium_text::FontTexture;
use glium_text::TextDisplay;
use cgmath::Matrix4;
use cgmath::SquareMatrix;
use cgmath::Rad;
use cgmath::PerspectiveFov;

use shared::tick_time::TickInstant;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::ViewDir;
use shared::consts::Y_FOV;
use shared::consts::OPTIMAL_SCREEN_RATIO;
use shared::consts::Z_NEAR;
use shared::consts::Z_FAR;

use self::visual_world::VisualWorld;
use self::visual_world::VisualCharacter;

#[derive(Copy, Clone)]
struct MyVertex {
    position: [f32; 3],
}

implement_vertex!(MyVertex, position);

struct VertexObject {
    vertex_buffer: glium::VertexBuffer<MyVertex>,
    index_buffer: glium::IndexBuffer<u32>,
}

pub struct Graphics {
    character_head_vo: VertexObject,
    text_system: TextSystem,
    font: FontTexture,
    program: glium::program::Program,
    background_program: glium::program::Program,
    perspective_matrix: Matrix4<f32>,
    debug_text_matrix: [[f32; 4]; 4],
    last_visual_world: VisualWorld,
    last_tick: u64,
    current_visual_world: VisualWorld,
    current_tick: u64,
    mix_world: VisualWorld,
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        // program
        let program = glium::Program::from_source(
            display,
            &Self::load_shader_source("shader_src/vertex_shader.vert"),
            &Self::load_shader_source("shader_src/fragment_shader.frag"),
            Some(&Self::load_shader_source("shader_src/geometry_shader.geo")),
        ).unwrap();

        // background program
        let background_program = glium::Program::from_source(
            display,
            &Self::load_shader_source("shader_src/background.vert"),
            &Self::load_shader_source("shader_src/background.frag"),
            None
        ).unwrap();

        let font_file = File::open("SourceCodeVariable-Roman.ttf").expect("Could not load font!");
        let font = FontTexture::new(display, &font_file, 35).unwrap();

        Graphics {
            character_head_vo: Self::build_character_head(display),
            text_system: TextSystem::new(display),
            font,
            program,
            background_program,
            debug_text_matrix: Matrix4::identity().into(),
            perspective_matrix: Matrix4::identity(),
            current_visual_world: VisualWorld::new(),
            current_tick: 0,
            last_visual_world: VisualWorld::new(), // TODO what about the first frame?
            last_tick: 0,
            mix_world: VisualWorld::new(),
        }
    }

    fn build_character_head(display: &Display) -> VertexObject {
        let z = 0.1;
        let width = 0.2;
        let height = 0.3;
        let depth = 0.2;

        let left = width / 2.0;
        let right = -width / 2.0;
        let top = height / 2.0 + z;
        let bottom = -height / 2.0 + z;
        let front = depth / 2.0;
        let back = -depth / 2.0;

        let vertex_data = &[
            MyVertex { position: [back,  left,  bottom] },
            MyVertex { position: [back,  right, bottom] },
            MyVertex { position: [back,  right, top] },
            MyVertex { position: [back,  left,  top] },
            MyVertex { position: [front, left,  bottom] },
            MyVertex { position: [front, right, bottom] },
            MyVertex { position: [front, right, top] },
            MyVertex { position: [front, left,  top] },
        ];
        let vertex_buffer = glium::VertexBuffer::new(display, vertex_data).unwrap();

        let index_data = &[
            0, 1, 2,
            0, 2, 3,
            1, 5, 6,
            1, 6, 2,
            5, 4, 7,
            5, 7, 6,
            4, 0, 3,
            4, 3, 7,
            0, 4, 5,
            0, 5, 1,
            3, 2, 6,
            3, 6, 7u32,
        ];
        let index_buffer = glium::IndexBuffer::new(
            display,
            PrimitiveType::TrianglesList,
            index_data
        ).unwrap();

        VertexObject {
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn draw(&mut self, current_model: &Model, predicted_world: &World, my_player_id: u64,
                view_dir: Option<ViewDir>, tick_instant: TickInstant, display: &Display) {
        let current_world = current_model.world();
        let my_character_id = current_model.player(my_player_id)
                .and_then(|p| p.character_id());
        if self.current_tick != tick_instant.tick {
            self.last_tick = self.current_tick;
            mem::swap(&mut self.last_visual_world, &mut self.current_visual_world);
        }
        self.current_tick = tick_instant.tick;
        self.current_visual_world.rebuild(my_character_id, current_world, predicted_world);

        let tick_diff = (self.current_tick - self.last_tick) as f64;
        self.mix_world.remix(
            &self.current_visual_world,
            &self.last_visual_world,
            (tick_diff - 1.0 + tick_instant.intra_tick) / tick_diff
        );

        let character = if let Some(c) = my_character_id.and_then(
            |id| self.current_visual_world.character(id)
        ) {
            c
        } else {
            return
        };

        let mut yaw = character.yaw();
        let mut pitch = character.pitch();

        // overwrite with direct camera
        if let Some(vd) = view_dir {
            yaw = vd.yaw().rad_f32();
            pitch = vd.pitch().rad_f32();
        }

        // world cs to character cs
        let world_to_character_matrix =
            Matrix4::from_angle_y(Rad(pitch as f32))
            * Matrix4::from_angle_z(Rad(-yaw as f32))
            * Matrix4::from_translation(-character.pos());

        // world cs to screen cs
        let world_to_screen_matrix = self.perspective_matrix * world_to_character_matrix;

        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_depth(1.0);

        self.draw_background(&mut frame, &world_to_screen_matrix);

        for (id, character) in self.mix_world.characters() {
            if Some(*id) == my_character_id {
                continue;
            }
            self.draw_character(character, &mut frame, &world_to_screen_matrix);
        }

        self.draw_debug_info(&mut frame);

        frame.finish().unwrap();
    }

    fn draw_background(&self, frame: &mut Frame, world_to_screen_matrix: &Matrix4<f32>) {
        let screen_to_world_matrix_uniform: [[f32; 4]; 4]
            = world_to_screen_matrix.invert().unwrap().into();
        let background_uniforms = uniform! {
            screen_to_world_matrix: screen_to_world_matrix_uniform
        };

        frame.draw(
            EmptyVertexAttributes { len: 4 },
            &NoIndices(PrimitiveType::TriangleStrip),
            &self.background_program,
            &background_uniforms,
            &Default::default(),
        ).unwrap();
    }

    fn draw_character(&self, character: &VisualCharacter,
                      frame: &mut Frame, world_to_screen_matrix: &Matrix4<f32>) {
        // character cs to world cs
        let character_to_world_matrix = Matrix4::from_translation(character.pos().into());
        // head cs to world cs
        let head_to_world_matrix = character_to_world_matrix
            * Matrix4::from_angle_z(Rad(character.yaw() as f32))
            * Matrix4::from_angle_y(Rad(-character.pitch() as f32));

        // uniforms
        let head_to_world_matrix_uniform: [[f32; 4]; 4] = head_to_world_matrix.into();
        let world_to_screen_matrix_uniform: [[f32; 4]; 4] = (*world_to_screen_matrix).into();
        let uniforms = uniform! {
            object_to_world_matrix:      head_to_world_matrix_uniform,
            world_to_screen_matrix:      world_to_screen_matrix_uniform,
            ambient_light_color:         [0.30, 0.30, 0.35f32],
            directional_light_dir:       [1.00, 2.00, 3.00f32],
            directional_light_color:     [0.50, 0.50, 0.50f32],
        };

        // draw parameters
        let draw_parameters = DrawParameters {
            depth: Depth {
                test: glium::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };
        frame.draw(
            &self.character_head_vo.vertex_buffer,
            &self.character_head_vo.index_buffer,
            &self.program,
            &uniforms,
            &draw_parameters,
        ).unwrap();
    }

    fn draw_debug_info(&self, frame: &mut Frame) {
        let tick_text = format!("Tick: {}", self.current_tick);
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

    fn load_shader_source(file_name: &str) -> String {
        let file = File::open(file_name).expect("Could not load shader source!");
        let mut buffer_reader = BufReader::new(file);
        let mut shader_source = String::new();
        buffer_reader.read_to_string(&mut shader_source)
            .expect("Error while reading shader source!");
        shader_source
    }

    pub fn set_view_port(&mut self, width: u64, height: u64) {
        let ratio = if height != 0 {
            width as f64 / height as f64
        } else {
            0.0
        };
        self.build_perspective_matrix(ratio, Y_FOV);
        self.build_debug_text_matrix(ratio);
    }

    fn build_perspective_matrix(&mut self, mut screen_ratio: f64, mut y_fov: f64) {
        if screen_ratio <= 0.0 {
            screen_ratio = OPTIMAL_SCREEN_RATIO;
        }
        // perspective
        if screen_ratio > OPTIMAL_SCREEN_RATIO {
            y_fov = ((y_fov / 2.0).tan() * OPTIMAL_SCREEN_RATIO / screen_ratio).atan() * 2.0;
        }
        let projection = PerspectiveFov {
            fovy: Rad(y_fov as f32),
            aspect: screen_ratio as f32,
            near: Z_NEAR as f32,
            far: Z_FAR as f32,
        };
        let projection_matrix: Matrix4<f32> = projection.into();
        self.perspective_matrix = projection_matrix
            * Matrix4::from_angle_y(Rad((PI / 2.0) as f32))
            * Matrix4::from_angle_x(Rad((-PI / 2.0) as f32));
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
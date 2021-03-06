mod visual_world;

use std::io::Read;
use std::f64::consts::PI;
use std::mem;
use std::fs::File;
use std::io::BufReader;

use glium;
use glium::Display;
use glium::Surface;
use glium::Frame;
use glium::program::Program;
use glium::draw_parameters::Depth;
use glium::draw_parameters::DrawParameters;
use glium::vertex::EmptyVertexAttributes;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
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

pub struct ModelGraphics {
    character_head_vo: VertexObject,
    program: Program,
    background_program: Program,
    perspective_matrix: Matrix4<f32>,
    last_visual_world: VisualWorld,
    last_tick: u64,
    current_visual_world: VisualWorld,
    current_tick: u64,
    mix_world: VisualWorld,
}

impl ModelGraphics {
    pub fn new(display: &Display) -> ModelGraphics {
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

        ModelGraphics {
            character_head_vo: Self::build_character_head(display),
            program,
            background_program,
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
                view_dir: Option<ViewDir>, tick_instant: TickInstant, frame: &mut Frame) {
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

        self.draw_background(frame, &world_to_screen_matrix);

        for (id, character) in self.mix_world.characters() {
            if Some(*id) == my_character_id {
                continue;
            }
            self.draw_character(character, frame, &world_to_screen_matrix);
        }
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

    fn load_shader_source(file_name: &str) -> String {
        let file = File::open(file_name).expect("Could not load shader source!");
        let mut buffer_reader = BufReader::new(file);
        let mut shader_source = String::new();
        buffer_reader.read_to_string(&mut shader_source)
            .expect("Error while reading shader source!");
        shader_source
    }

    pub fn set_screen_ratio(&mut self, screen_ratio: f64) {
        self.build_perspective_matrix(screen_ratio);
    }

    fn build_perspective_matrix(&mut self, mut screen_ratio: f64) {
        if screen_ratio <= 0.0 {
            screen_ratio = OPTIMAL_SCREEN_RATIO;
        }
        // perspective
        let y_fov = if screen_ratio > OPTIMAL_SCREEN_RATIO {
            ((Y_FOV / 2.0).tan() * OPTIMAL_SCREEN_RATIO / screen_ratio).atan() * 2.0
        } else {
            Y_FOV
        };
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
}
mod interpolate_world;

use std;
use std::io::Read;
use std::f64::consts::PI;
use std::mem;

use glium;
use glium::backend::glutin::Display;
use glium::Surface;
use glium::draw_parameters::Depth;
use glium::draw_parameters::DrawParameters;
use glium::vertex::EmptyVertexAttributes;
use glium::index::NoIndices;
use cgmath::Matrix4;
use cgmath::Vector3;
use cgmath::SquareMatrix;
use cgmath::Rad;
use cgmath::PerspectiveFov;

use shared::model::world::World;
use shared::consts::Y_FOV;
use shared::consts::OPTIMAL_SCREEN_RATIO;
use shared::consts::Z_NEAR;
use shared::consts::Z_FAR;
use self::interpolate_world::VisualWorld;

#[derive(Copy, Clone)]
struct MyVertex {
    position: [f32; 3],
}

implement_vertex!(MyVertex, position);

pub struct Graphics {
    vertex_buffer: glium::VertexBuffer<MyVertex>,
    index_buffer: glium::IndexBuffer<u32>,
    program: glium::program::Program,
    background_program: glium::program::Program,
    perspective_matrix: Matrix4<f32>,
    last_visual_world: VisualWorld,
    last_tick: u64,
    current_visual_world: VisualWorld,
    current_tick: u64,
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        // program
        let program = glium::Program::from_source(
            display,
            &Self::load_shader_source("shader_src/vertex_shader.vert"),
            &Self::load_shader_source("shader_src/fragment_shader.frag"),
            None
        ).unwrap();

        // background program
        let background_program = glium::Program::from_source(
            display,
            &Self::load_shader_source("shader_src/background.vert"),
            &Self::load_shader_source("shader_src/background.frag"),
            None
        ).unwrap();

        // vertex buffer
        let vertex_data = &[
            MyVertex {
                position: [0.5, -0.5, 0.2]
            },
            MyVertex {
                position: [0.5,  0.5, 0.2]
            },
            MyVertex {
                position: [0.5, -0.5, 1.2]
            },
            MyVertex {
                position: [0.5,  0.5, 1.2]
            },
        ];
        let vertex_buffer = glium::VertexBuffer::new(display, vertex_data).unwrap();

        // index buffer
        let index_data = &[
            0u32, 3, 1,
            0, 2, 3,
        ];
        let index_buffer = glium::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            index_data
        ).unwrap();

        Graphics {
            vertex_buffer,
            index_buffer,
            program,
            background_program,
            perspective_matrix: Matrix4::identity(),
            current_visual_world: VisualWorld::from(&World::new()),
            current_tick: 0,
            last_visual_world: VisualWorld::from(&World::new()),
            last_tick: 0,
        }
    }

    pub fn draw(&mut self, current_world: &World, predicted_world: &World,
                tick: u64, intra_tick: f64, display: &Display) {
        if self.current_tick != tick {
            self.last_tick = self.current_tick;
            mem::swap(&mut self.last_visual_world, &mut self.current_visual_world);
        }
        self.current_tick = tick;
        self.current_visual_world = VisualWorld::build(current_world, predicted_world);

        let tick_diff = (self.current_tick - self.last_tick) as f64;
        let inter_visual_world = self.last_visual_world.interpolate(
            &self.current_visual_world,
            (tick_diff - 1.0 + intra_tick) / tick_diff
        );

        // world cs to character cs
        let cp = inter_visual_world.get_character().get_pos();
        let character_position = Vector3 {
            x: cp.0 as f32,
            y: cp.1 as f32,
            z: cp.2 as f32,
        };
        let yaw = inter_visual_world.get_character().get_yaw();
        let pitch = inter_visual_world.get_character().get_pitch();
        let inverse_character_matrix =
            Matrix4::from_angle_y(Rad(pitch as f32))
            * Matrix4::from_angle_z(Rad(-yaw as f32))
            * Matrix4::from_translation(-character_position);

        // object cs to global cs
        let object_position = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0f32,
        };
        let object_matrix = Matrix4::from_angle_z(Rad(0f32))
                * Matrix4::from_translation(object_position);

        // world cs to screen cs
        let world_to_screen_matrix = self.perspective_matrix * inverse_character_matrix;

        // object cs to screen cs
        let object_to_screen_matrix = world_to_screen_matrix * object_matrix;

        // uniforms
        let object_to_screen_matrix_uniform: [[f32; 4]; 4] = object_to_screen_matrix.into();
        let uniforms = uniform! { trafo_matrix: object_to_screen_matrix_uniform };
        let screen_to_world_matrix_uniform: [[f32; 4]; 4] = world_to_screen_matrix.invert().unwrap().into();
        let background_uniforms = uniform! { trafo_matrix: screen_to_world_matrix_uniform };

        // draw parameters
        let draw_parameters = DrawParameters {
            depth: Depth {
                test: glium::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // background transformation matrix
        let mut frame = display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_depth(1.0);
        frame.draw(
            EmptyVertexAttributes {len: 4},
            &NoIndices(glium::index::PrimitiveType::TriangleStrip),
            &self.background_program,
            &background_uniforms,
            &Default::default(),
        ).unwrap();
        frame.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &uniforms,
            &draw_parameters,
        ).unwrap();
        frame.finish().unwrap();
    }

    fn load_shader_source(file_name: &str) -> String {
        let file = std::fs::File::open(file_name).expect("Could not load shader source!");
        let mut buffer_reader = std::io::BufReader::new(file);
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
            fovy: Rad(y_fov as f32), // should be y
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
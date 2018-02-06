extern crate cgmath;

use super::glium;
use super::glium::backend::glutin::Display;
use self::cgmath::Matrix4;

use model::world::World;

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
}

impl Graphics {
    pub fn new(display: &Display) -> Graphics {
        use self::cgmath::SquareMatrix;

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
        }
    }

    pub fn draw(&mut self, world: &World, display: &Display) {
        use self::glium::Surface;
        use self::glium::draw_parameters;
        use self::glium::vertex::EmptyVertexAttributes;
        use self::glium::index::NoIndices;

        use self::cgmath::Rad;
        use self::cgmath::Matrix4;
        use self::cgmath::Vector3;
        use self::cgmath::SquareMatrix;

        // world cs to character cs
        let cp = world.get_character().get_pos();
        let character_position = Vector3 {
            x: cp.0 as f32,
            y: cp.1 as f32,
            z: cp.2 as f32,
        };
        let inverse_character_matrix =
            Matrix4::from_angle_y(Rad(world.get_character().get_pitch() as f32))
            * Matrix4::from_angle_z(Rad(-world.get_character().get_yaw() as f32))
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
        let draw_parameters = draw_parameters::DrawParameters {
            depth: draw_parameters::Depth {
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
        use std;
        use std::io::Read;

        let file = std::fs::File::open(file_name).expect("Could not load shader source!");
        let mut buffer_reader = std::io::BufReader::new(file);
        let mut shader_source = String::new();
        buffer_reader.read_to_string(&mut shader_source)
            .expect("Error while reading shader source!");
        shader_source
    }

    pub fn set_view_port(&mut self, width: u64, height: u64) {
        use shared::consts::Y_FOV;
        self.build_perspective_matrix(width as f64 / height as f64, Y_FOV);
    }

    fn build_perspective_matrix(&mut self, screen_ratio: f64, mut y_fov: f64) {
        use std::f32::consts::PI;
        use self::cgmath::PerspectiveFov;
        use self::cgmath::Rad;
        use shared::consts::OPTIMAL_SCREEN_RATIO;
        use shared::consts::Z_NEAR;
        use shared::consts::Z_FAR;

        // perspective
        if screen_ratio >= OPTIMAL_SCREEN_RATIO {
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
            * Matrix4::from_angle_y(Rad(PI / 2.0))
            * Matrix4::from_angle_x(Rad(-PI / 2.0));
    }
}
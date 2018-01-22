extern crate cgmath;

use std::f32::consts::PI;

use super::glium;

use ::world::World;

#[derive(Copy, Clone)]
struct MyVertex {
    position: [f32; 3],
}

glium::implement_vertex!(MyVertex, position);

pub struct Graphics {
    display: glium::backend::glutin::Display,
    vertex_buffer: glium::VertexBuffer<MyVertex>,
    index_buffer: glium::IndexBuffer<u32>,
    program: glium::program::Program,
}

impl Graphics {
    pub fn new(display: glium::Display) -> Graphics {
        // program
        let vertex_shader_source = Self::load_shader_source("shader_src/vertex_shader.vert");
        let fragment_shader_source = Self::load_shader_source("shader_src/fragment_shader.frag");
        let program = glium::Program::from_source(
            &display,
            &vertex_shader_source,
            &fragment_shader_source,
            None
        ).unwrap();

        // vertex buffer
        let vertex_data = &[
            MyVertex {
                position: [0.5, -0.5, -0.5]
            },
            MyVertex {
                position: [0.5,  0.5, -0.5]
            },
            MyVertex {
                position: [0.5, -0.5,  0.5]
            },
            MyVertex {
                position: [0.5,  0.5,  0.5]
            },
        ];
        let vertex_buffer = glium::VertexBuffer::new(&display, vertex_data).unwrap();

        // index buffer
        let index_data = &[
            0u32, 3, 1,
            0, 2, 3,
        ];
        let index_buffer = glium::IndexBuffer::new(
            &display,
            glium::index::PrimitiveType::TrianglesList,
            index_data
        ).unwrap();

        Graphics {
            display,
            vertex_buffer,
            index_buffer,
            program,
        }
    }

    pub fn draw(&mut self, world: &World) { // the world probably shouldn't be a parameter
        use self::glium::Surface;
        use self::glium::uniform;

        use self::cgmath::Matrix4;
        use self::cgmath::Vector3;
        use self::cgmath::Rad;
        use self::cgmath::PerspectiveFov;

        // global cs to character cs
        let character_position = Vector3 {
            x:-10.0,
            y: 0.0,
            z: 0.0f32,
        };
        let character_yaw = 0.0;
        let character_pitch = 0.0;
        let inverse_character_matrix = Matrix4::from_translation(-character_position)
                * Matrix4::from_angle_y(Rad(-character_pitch))
                * Matrix4::from_angle_z(Rad(-character_yaw));

        // object cs to global cs
        let object_position = Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0f32,
        };
        let object_matrix = Matrix4::from_angle_z(Rad(world.get_angle() as f32))
                * Matrix4::from_translation(object_position);

        // character cs to eye cs
        let inverse_eye_matrix = Matrix4::from_angle_y(Rad(PI / 2.0))
            * Matrix4::from_angle_x(Rad(-PI / 2.0));

        // object cs to eye cs
        let model_matrix = inverse_eye_matrix * inverse_character_matrix * object_matrix;

        // perspective
        let aspect_ratio = 16.0 / 9.0f32;
        let x_fov = PI / 4.0;
        let z_near = 0.1;
        let z_far = 100.0;
        let perspective = PerspectiveFov {
            fovy: Rad(x_fov), // should be y
            aspect: aspect_ratio,
            near: z_near,
            far: z_far,
        };
        let perspective_matrix: Matrix4<f32> = perspective.into();

        // overall transformation
        let transformation_matrix = perspective_matrix * model_matrix;
        let transformation_matrix_uniform: [[f32; 4]; 4] = transformation_matrix.into();
        let uniforms = uniform! {
            trafo_matrix: transformation_matrix_uniform,
        };

        let mut frame = self.display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        frame.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &uniforms,
            &Default::default(),
        ).unwrap();
        frame.finish().unwrap();
    }

    fn load_shader_source(file_name: &str) -> String {
        use std;
        use std::io::Read;

        let file = std::fs::File::open(file_name)
            .expect("Could not load vertex shader source!");
        let mut vertex_buffer_reader = std::io::BufReader::new(file);
        let mut vertex_shader_source = String::new();
        vertex_buffer_reader.read_to_string(&mut vertex_shader_source)
            .expect("Error while reading vertex shader source!");
        vertex_shader_source
    }
}
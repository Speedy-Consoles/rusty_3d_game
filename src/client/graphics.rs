use super::glium;
use super::glutin;

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
    uniforms: glium::uniforms::EmptyUniforms,
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

        // uniforms
        let uniforms = glium::uniforms::EmptyUniforms;

        // vertex buffer
        let vertex_data = &[
            MyVertex {
                position: [-0.5, -0.5, -0.5]
            },
            MyVertex {
                position: [ 0.5, -0.5, -0.5]
            },
            MyVertex {
                position: [-0.5,  0.5, -0.5]
            },
            MyVertex {
                position: [ 0.5,  0.5, -0.5]
            },
        ];
        let vertex_buffer = glium::VertexBuffer::new(&display, vertex_data).unwrap();

        // index buffer
        let index_data = &[
            0u32, 1, 3,
            0, 3, 2,
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
            uniforms,
        }
    }

    pub fn draw(&mut self) {
        use self::glium::Surface;

        let mut draw_parameters = glium::DrawParameters::default();

        //println!("{:?}", self.graphics.display.gl_window().get_inner_size().unwrap());
        let mut frame = self.display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        frame.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &self.uniforms,
            &draw_parameters
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
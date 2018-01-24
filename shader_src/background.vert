#version 140

const vec2 quad_vertices[4] = vec2[4](vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
const vec2 quad_vertices_sym[4] = vec2[4](vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0));

out vec4 position;

void main() {
    gl_Position = vec4(quad_vertices_sym[gl_VertexID], 1.0, 1.0);
    position = vec4(quad_vertices[gl_VertexID], 1.0, 1.0);
}

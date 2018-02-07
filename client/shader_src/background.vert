#version 140

uniform mat4 trafo_matrix;

const vec2 quad_vertices_sym[4] = vec2[4](vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0));

out vec2 position;

void main() {
    gl_Position = vec4(quad_vertices_sym[gl_VertexID], 0.0, 1.0);
    position = quad_vertices_sym[gl_VertexID];
}

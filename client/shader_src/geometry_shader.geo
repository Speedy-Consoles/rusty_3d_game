#version 400

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;

in vec4 screen_position[];
in vec4 world_position[];

out vec3 normal;

void main() {
    vec3 edge1 = world_position[1].xyz - world_position[0].xyz;
    vec3 edge2 = world_position[2].xyz - world_position[0].xyz;
    normal = normalize(cross(edge1, edge2));

    gl_Position = screen_position[0];
    EmitVertex();

    gl_Position = screen_position[1];
    EmitVertex();

    gl_Position = screen_position[2];
    EmitVertex();
}
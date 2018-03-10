#version 400

uniform mat4 world_to_screen_matrix;
uniform mat4 object_to_world_matrix;

in vec3 position;

out vec4 world_position;
out vec4 screen_position;

void main() {
    world_position = object_to_world_matrix * vec4(position, 1.0);
    screen_position = world_to_screen_matrix * world_position;
}

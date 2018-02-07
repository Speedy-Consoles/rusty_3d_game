#version 140

in vec3 position;

uniform mat4 trafo_matrix;

void main() {
    gl_Position = trafo_matrix * vec4(position, 1.0);
}

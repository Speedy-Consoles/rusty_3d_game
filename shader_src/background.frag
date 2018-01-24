#version 140

uniform mat4 trafo_matrix;

in vec4 position;
out vec4 color;

void main() {
    color = position;//vec4(0.0, 0.0, 1.0, 1.0);
}
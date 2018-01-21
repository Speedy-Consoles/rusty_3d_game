#version 140

in vec3 position;

uniform mat4 modelMatrix;
uniform mat4 perspectiveMatrix;

void main() {
    gl_Position = vec4(position, 1.0);
}

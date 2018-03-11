#version 400

uniform vec3 ambient_light_color;
uniform vec3 directional_light_dir;
uniform vec3 directional_light_color;

vec3 normalized_directional_light_dir = normalize(directional_light_dir);

in vec3 normal;

out vec4 color;

void main() {
    vec3 light = max(0.0, -dot(normal, normalized_directional_light_dir)) * directional_light_color;
    light += ambient_light_color;
    color = vec4(vec3(1.0, 0.0, 0.0) * light, 1.0);
}
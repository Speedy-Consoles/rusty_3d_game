#version 140

uniform mat4 trafo_matrix;

const vec4 color1 = vec4(vec3(210.0, 163.0, 36.0) / 255.0, 1.0);
const vec4 color2 = vec4(vec3(72.0, 67.0, 54.0) / 255.0, 1.0);

in vec2 position;
out vec4 color;

void main() {
    vec2 p = position;
    mat4 t = trafo_matrix;
    color = vec4(1.0, 0.0, 1.0, 1.0);
    if (trafo_matrix[2][2] == 0.0)
        return;
    color = vec4(0.0, 1.0, 1.0, 1.0);
    float c = (-t[0][2] * p.x - t[1][2] * p.y - t[3][2]) / t[2][2];
    float divisor = t[0][3] * p.x + t[1][3] * p.y + t[2][3] * c + t[3][3];
    if (divisor <= 0.0)
        return;
    float d = 1.0 / divisor;
    vec4 world_coords = trafo_matrix * vec4(p * d, c * d, d);
    vec2 tile_value = mod(world_coords.xy / 10.0, 1.0);
    if (tile_value.x > 0.5 && tile_value.y > 0.5)
        color = color1;
    else
        color = color2;
}
#version 140

uniform mat4 trafo_matrix;

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
    if (divisor == 0.0)
        return;
    float d = 1.0 / divisor;
    vec4 world_coords = trafo_matrix * vec4(p, c, d);
    color = vec4(world_coords.xy / 1000.0, 0.0, 1.0);
    //color = vec4(0.0, 1.0, 0.0, 1.0);
}
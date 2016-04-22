#version 140

in  vec2 tx_coord;
out vec4 color;

uniform sampler2D atlas;

void main() {
    float u = tx_coord.x + (0.5 / 1024.0);
    float v = tx_coord.y + (0.5 / 1024.0);
    vec2 uv = vec2(u,v);
    color = texture(atlas, uv);
}

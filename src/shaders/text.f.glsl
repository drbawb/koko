#version 140

in  vec2 tx_coord;
out vec4 color;

uniform sampler2D atlas;

void main() {
    color = texture(atlas, tx_coord);
}

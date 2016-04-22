#version 140

in  vec2 tx_coord;
out vec4 color;

uniform vec2 c_ofs;
uniform sampler2DArray atlas_arr;

void main() {
    color = texture(atlas_arr, vec3(tx_coord, c_ofs.y));
}

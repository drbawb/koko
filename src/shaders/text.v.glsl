#version 140

in  vec3 pos;
in  vec3 color;
out vec2 tx_coord;

uniform vec2    c_ofs;
uniform vec3    c_pos;
uniform vec3    w_ofs;
uniform float   scale;

void main() {
    mat4 projection = mat4(
        vec4(1.0, 0.0, 0.0, 0.0),
        vec4(0.0, 1.0, 0.0, 0.0),
        vec4(0.0, 0.0, 1.0, 0.0),
        vec4(0.0, 0.0, 0.0, 1.0)
    );

    mat4 center = mat4(
        vec4( 1.0,  0.0,  0.0,  0.0),
        vec4( 0.0,  1.0,  0.0,  0.0),
        vec4( 0.0,  0.0,  1.0,  0.0),
        vec4( 1.0, -1.0,  0.0,  1.0)
    );

    mat4 transchar = mat4(
        vec4(    1.0,     0.0,  0.0,  0.0),
        vec4(    0.0,     1.0,  0.0,  0.0),
        vec4(    0.0,     0.0,  1.0,  0.0),
        vec4(c_pos.x, c_pos.y,  0.0,  1.0)
    );

    mat4 transworld = mat4(
        vec4(    1.0,     0.0,  0.0,  0.0),
        vec4(    0.0,     1.0,  0.0,  0.0),
        vec4(    0.0,     0.0,  1.0,  0.0),
        vec4(w_ofs.x, w_ofs.y,  0.0,  1.0)
    );

    mat4 scale = mat4(
        vec4(scale,   0.0,   0.0,  0.0),
        vec4(  0.0, scale,   0.0,  0.0),
        vec4(  0.0,   0.0, scale,  0.0),
        vec4(  0.0,   0.0,   0.0,  1.0)
    );

    vec4 pos3d    = vec4(pos, 1.0);
    vec4 proj_pos = projection * transworld * scale * transchar * center * pos3d;
    gl_Position   = proj_pos;

    tx_coord = (pos3d.xy * vec2(0.5) + vec2(0.5)) + c_ofs;
}

#version 140

in  vec3 pos;
in  vec3 color;
out vec2 tx_coord;

uniform vec3      ofs;
uniform float   scale;
uniform float   timer;

void main() {
    mat4 projection = mat4(
        vec4(1.0, 0.0, 0.0, 0.0),
        vec4(0.0, 1.0, 0.0, 0.0),
        vec4(0.0, 0.0, 0.5, 0.5),
        vec4(0.0, 0.0, 0.0, 1.0)
    );

    mat4 rotation = mat4(
        vec4(1.0,         0.0,         0.0, 0.0),
        vec4(0.0,  cos(timer), -sin(timer), 0.0),
        vec4(0.0,  sin(timer),  cos(timer), 0.0),
        vec4(0.0,         0.0,         0.0, 1.0)
    );

    mat4 translate = mat4(
        vec4(  1.0,   0.0,  0.0,  0.0),
        vec4(  0.0,   1.0,  0.0,  0.0),
        vec4(  0.0,   0.0,  1.0,  0.0),
        vec4(ofs.x, ofs.y,  0.0,  1.0)
    );

    mat4 scale = mat4(
        vec4(scale,   0.0,   0.0,  0.0),
        vec4(  0.0, scale,   0.0,  0.0),
        vec4(  0.0,   0.0, scale,  0.0),
        vec4(  0.0,   0.0,   0.0,  1.0)
    );

    vec4 pos3d     = vec4(pos, 1.0);
    vec4 proj_pos  = translate * projection * scale * pos3d;
    float perspective_factor = proj_pos.z * 0.5 + 1.0;
    
    gl_Position = proj_pos/perspective_factor;
    tx_coord = pos3d.xy * vec2(0.5) + vec2(0.5);
}

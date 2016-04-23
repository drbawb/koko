#version 140

in  vec3 pos;
in  vec3 color;
out vec4 px_color;
out float fade_factor;

uniform vec3      ofs;
uniform float   scale;

void main() {
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

    vec4 pos3d  = vec4(pos, 1.0);
    gl_Position = translate * scale * pos3d;;
    px_color    = vec4(color, 1.0);
}

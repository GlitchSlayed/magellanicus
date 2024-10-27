#version 450

layout(location = 0) in vec3 position;
layout(location = 0) out vec2 texture_coords;

void main() {
    gl_Position = vec4((position * 2.0) - 1.0, 1.0);
    switch(gl_VertexIndex) {
        case 0: texture_coords = vec2(0.0, 0.0); break;
        case 1: texture_coords = vec2(0.0, 1.0); break;
        case 2: texture_coords = vec2(1.0, 1.0); break;
        case 3: texture_coords = vec2(1.0, 0.0); break;
    }
}

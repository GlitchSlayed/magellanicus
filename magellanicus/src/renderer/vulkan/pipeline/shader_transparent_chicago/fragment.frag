#version 450

#include "shader_transparent_chicago_data.glsl"

layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 binormal;
layout(location = 3) in vec3 tangent;
layout(location = 4) in vec3 camera_position;
layout(location = 5) in vec3 vertex_position;

#define USE_FOG
#define USE_TANGENT
#include "../include/material.frag"
#include "../include/blend.frag"

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec2 texture_coordinates;

layout(set = 3, binding = 1) uniform sampler map_sampler;
layout(set = 3, binding = 2) uniform textureCube map0_cube;
layout(set = 3, binding = 3) uniform texture2D map0_2d;
layout(set = 3, binding = 4) uniform texture2D map1;
layout(set = 3, binding = 5) uniform texture2D map2;
layout(set = 3, binding = 6) uniform texture2D map3;



#define COLOR_FN_CURRENT 0
#define COLOR_FN_NEXT_MAP 1
#define COLOR_FN_MULTIPLY 2
#define COLOR_FN_DOUBLE_MULTIPLY 3
#define COLOR_FN_ADD 4
#define COLOR_FN_ADD_SIGNED_CURRENT 5
#define COLOR_FN_ADD_SIGNED_NEXT_MAP 6
#define COLOR_FN_SUBTRACT_CURRENT 7
#define COLOR_FN_SUBTRACT_NEXT_MAP 8
#define COLOR_FN_BLEND_CURRENT_ALPHA 9
#define COLOR_FN_BLEND_CURRENT_ALPHA_INVERSE 10
#define COLOR_FN_BLEND_NEXT_MAP_ALPHA 11
#define COLOR_FN_BLEND_NEXT_MAP_ALPHA_INVERSE 12

float calculate_color(float current, float map, float map_alpha, float current_alpha, uint color_function) {
    switch(color_function) {
        case COLOR_FN_CURRENT: return current;
        case COLOR_FN_NEXT_MAP: return map;
        case COLOR_FN_MULTIPLY: return current * map;
        case COLOR_FN_DOUBLE_MULTIPLY: return current * map * 2.0;
        case COLOR_FN_ADD: return current + map;
        case COLOR_FN_ADD_SIGNED_CURRENT: return current + -current; // ???
        case COLOR_FN_ADD_SIGNED_NEXT_MAP: return current + -map; // ???
        case COLOR_FN_SUBTRACT_CURRENT: return current - current; // ???
        case COLOR_FN_SUBTRACT_NEXT_MAP: return current - map; // ???
        case COLOR_FN_BLEND_CURRENT_ALPHA: return mix(current, map, current_alpha);
        case COLOR_FN_BLEND_CURRENT_ALPHA_INVERSE: return mix(map, current, current_alpha);
        case COLOR_FN_BLEND_NEXT_MAP_ALPHA: return mix(current, map, map_alpha);
        case COLOR_FN_BLEND_NEXT_MAP_ALPHA_INVERSE: return mix(map, current, map_alpha);

        default: return 1.0;
    }
}

vec4 calculate_colors(
    vec4 current_color,
    vec4 map_color,
    uint color_function,
    uint alpha_function,
    uint map_index
) {
    if(map_index >= shader_transparent_chicago_data.map_count) {
        return current_color;
    }

    uint alpha_replicate = shader_transparent_chicago_data.alpha_replicate & (1 << (map_index - 1));
    if(alpha_replicate != 0) {
        map_color = map_color.aaaa;
    }

    vec4 color_out;
    color_out.r = calculate_color(current_color.r, map_color.r, map_color.a, current_color.a, color_function);
    color_out.g = calculate_color(current_color.g, map_color.g, map_color.a, current_color.a, color_function);
    color_out.b = calculate_color(current_color.b, map_color.b, map_color.a, current_color.a, color_function);
    color_out.a = calculate_color(current_color.a, map_color.a, map_color.a, current_color.a, alpha_function);
    return color_out;
}

void main() {
    vec4 map0_color;

    if(shader_transparent_chicago_data.first_map_type == 0) {
        map0_color = texture(
           sampler2D(map0_2d, map_sampler),
           (texture_coordinates + shader_transparent_chicago_data.map0_uv) * shader_transparent_chicago_data.map0_scale
        );
    }
    else {
        vec3 asdf = calculate_world_normal(vec3(0.0, 0.0, 1.0));
        map0_color = texture(
            samplerCube(map0_cube, map_sampler),
            (asdf + vec3(shader_transparent_chicago_data.map0_uv, 1.0)) * vec3(shader_transparent_chicago_data.map0_scale, 1.0)
        );
    }

    vec4 map1_color = texture(
        sampler2D(map1, map_sampler),
        (texture_coordinates + shader_transparent_chicago_data.map1_uv) * shader_transparent_chicago_data.map1_scale
    );
    vec4 map2_color = texture(
        sampler2D(map2, map_sampler),
        (texture_coordinates + shader_transparent_chicago_data.map2_uv) * shader_transparent_chicago_data.map2_scale
    );
    vec4 map3_color = texture(
        sampler2D(map3, map_sampler),
        (texture_coordinates + shader_transparent_chicago_data.map3_uv) * shader_transparent_chicago_data.map3_scale
    );

    vec4 current_color = map0_color;

    current_color = calculate_colors(
        current_color,
        map1_color,
        shader_transparent_chicago_data.map0_color_function,
        shader_transparent_chicago_data.map0_alpha_function,
        1
    );

    current_color = calculate_colors(
        current_color,
        map2_color,
        shader_transparent_chicago_data.map1_color_function,
        shader_transparent_chicago_data.map1_alpha_function,
        2
    );

    current_color = calculate_colors(
        current_color,
        map3_color,
        shader_transparent_chicago_data.map2_color_function,
        shader_transparent_chicago_data.map2_alpha_function,
        3
    );

    vec3 camera_difference = camera_position - vertex_position;
    float distance_from_camera = distance(camera_position, vertex_position);
    float inverse_density = 1.0 - calculate_fog_density(distance_from_camera);

    current_color.a *= inverse_density;

    if(shader_transparent_chicago_data.premultiply != 0) {
        current_color.rgb *= inverse_density;
    }

    f_color = clamp(current_color, vec4(0.0), vec4(1.0));
}

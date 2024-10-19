layout(set = 3, binding = 0) uniform ShaderTransparentChicagoData {
    vec2 map0_uv;
    vec2 map0_scale;
    uint map0_color_function;
    uint map0_alpha_function;

    vec2 map1_uv;
    vec2 map1_scale;
    uint map1_color_function;
    uint map1_alpha_function;

    vec2 map2_uv;
    vec2 map2_scale;
    uint map2_color_function;
    uint map2_alpha_function;

    vec2 map3_uv;
    vec2 map3_scale;
    uint map3_color_function;
    uint map3_alpha_function;

    uint first_map_type;
    uint map_count;
    uint premultiply;
    uint alpha_replicate;
} shader_transparent_chicago_data;

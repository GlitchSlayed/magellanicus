#ifdef USE_LIGHTMAPS
layout(set = 1, binding = 0) uniform sampler lightmap_sampler;
layout(set = 1, binding = 1) uniform texture2D lightmap_texture;
#endif

#ifdef USE_FOG
layout(set = 2, binding = 0) uniform FogData {
    vec4 sky_fog_color;
    float sky_fog_from;
    float sky_fog_to;
    float min_opacity;
    float max_opacity;
} sky_fog_data;

float calculate_fog_density(float distance_from_camera) {
    float clamped = clamp(distance_from_camera, sky_fog_data.sky_fog_from, sky_fog_data.sky_fog_to);

    // This is a pretty close approximation of the algorithm used for fog from planar fog density
    //
    // used https://tools.softinery.com/CurveFitter/ with 4th order polynomial to find these values
    float x = (clamped - sky_fog_data.sky_fog_from) / (sky_fog_data.sky_fog_to - sky_fog_data.sky_fog_from);
    float x2 = x*x;
    float x3 = x*x2;
    float x4 = x*x3;
    float y = 1.47892 * x4 - 5.18239 * x3 + 5.00783 * x2 - 0.30246 * x - 0.00072;

    // This can be slightly outside of 0.0-1.0, however, when x is close to 0.0 or 1.0.
    float interpolation = clamp(y, 0.0, 1.0);

    return interpolation * sky_fog_data.max_opacity;
}

vec3 apply_fog(float distance_from_camera, vec3 color) {
    float fog_density = calculate_fog_density(distance_from_camera);
    return mix(color, sky_fog_data.sky_fog_color.rgb, fog_density);
}
#endif

#ifdef USE_TANGENT
vec3 calculate_world_normal(vec3 base) {
    return base.xxx * tangent + base.yyy * binormal + base.zzz * normal;
}
#endif

#version 460

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec2 out_uv;

layout(set = 0, binding = 0) uniform sampler s;
layout(set = 0, binding = 1) uniform texture2D tex;

layout(push_constant) uniform PushConstantData {
    int visible_layer;
} pc;

// taken from https://www.shadertoy.com/view/WlfXRN
vec3 plasma(float t) {
    const vec3 c0 = vec3(0.05873234392399702, 0.02333670892565664, 0.5433401826748754);
    const vec3 c1 = vec3(2.176514634195958, 0.2383834171260182, 0.7539604599784036);
    const vec3 c2 = vec3(-2.689460476458034, -7.455851135738909, 3.110799939717086);
    const vec3 c3 = vec3(6.130348345893603, 42.3461881477227, -28.51885465332158);
    const vec3 c4 = vec3(-11.10743619062271, -82.66631109428045, 60.13984767418263);
    const vec3 c5 = vec3(10.02306557647065, 71.41361770095349, -54.07218655560067);
    const vec3 c6 = vec3(-3.658713842777788, -22.93153465461149, 18.19190778539828);

    return c0 + t * (c1 + t * (c2 + t * (c3 + t * (c4 + t * (c5 + t * c6)))));
}

// taken from https://www.shadertoy.com/view/Nd3fR2
vec3 turbo(float t) {
    t = clamp(t, 0.0, 1.0);
    return clamp(vec3((0.192919 + t * (1.618437 + t * (-39.426098 + t * (737.420549 + t * (-6489.216487 + t * (28921.755478 + t * (-72384.553891 + t * (107076.097978 + t * (-93276.212113 + t * (44337.286143 + t * -8884.508085)))))))))),
            (0.101988 + t * (1.859131 + t * (7.108520 + t * (-20.179546 + t * 11.147684)))),
            (0.253316 + t * (4.858570 + t * (55.191710 + t * (-803.379980 + t * (4477.461997 + t * (-14496.039745 + t * (28438.311669 + t * (-32796.884355 + t * (20328.068712 + t * -5210.826342)))))))))), 0.0, 1.0);
}

vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec3 rgb2hsv(vec3 c)
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

void main() {
    vec4 color = texture(sampler2D(tex, s), out_uv);
    int layer = pc.visible_layer;

    float layers[4] = {
            color.r, // Real
            color.g, // Imaginary
            (color.r * color.r + color.g * color.a), // Probability
            color.b, // Potential
        };

    switch (layer) {
        case 4: {
            f_color = vec4(hsv2rgb(vec3(atan(color.g, color.r), 1.0, layers[2])) + min(1.0, layers[3]) * plasma(layers[3]), 1.0);
        } break;
        default: {
            f_color = vec4(turbo(layers[layer]) + min(1.0, layers[3]) * plasma(layers[3]), 1.0);
        } break;
    }
}

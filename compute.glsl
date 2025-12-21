#version 460

layout (local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout (set = 0, binding = 0, rgba8) uniform image2D img;

void main() {
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), vec4(0.0, norm_coordinates.x, norm_coordinates.y, 1.0));
}
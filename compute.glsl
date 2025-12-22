#version 460

layout (local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout (push_constant) uniform PushConstantData {
    int brush_x;
    int brush_y;
    int brush_enabled;
    int brush_radius;
} pc;

layout (set = 0, binding = 0, rgba32f) uniform image2D img;


float U(int x, int y) {
    if (x < 0 || x >= imageSize(img).x || y < 0 || y >= imageSize(img).y)
        return 0.0;
    return imageLoad(img, ivec2(x, y)).r;
}

float U(ivec2 location) {
    return U(location.x, location.y);
}

float V(int x, int y) {
    if (x < 0 || x >= imageSize(img).x || y < 0 || y >= imageSize(img).y)
        return 0.0;
    return imageLoad(img, ivec2(x, y)).g;
}

float V(ivec2 location) {
    return V(location.x, location.y);
}

float du_dt(int x, int y) {
    float dx = 1.0;
    float dt = 0.25;

    float du_dx_0 = (U(x, y) - U(x-1, y)) / dx;
    float du_dx_1 = (U(x+1, y) - U(x, y)) / dx;

    float du_dy_0 = (U(x, y) - U(x, y-1)) / dx;
    float du_dy_1 = (U(x, y+1) - U(x, y)) / dx;

    float d2u_dx2 = (du_dx_1 - du_dx_0) / dx;
    float d2u_dy2 = (du_dy_1 - du_dy_0) / dx;

    float c = dt / dx; // Follow CFL
    return pow(c, 2) * (d2u_dx2 + d2u_dy2);
}

float du_dt(ivec2 location) {
    return du_dt(location.x, location.y);
}

void main() {
    ivec2 location = ivec2(gl_GlobalInvocationID.xy);
    int x = location.x;
    int y = location.y;

    ivec2 brush_pos = ivec2(pc.brush_x, pc.brush_y);
    int dist = pc.brush_radius;

    // imageStore(img, ivec2(gl_GlobalInvocationID.xy), vec4(norm_coordinates.x, 0.0, norm_coordinates.y, 1.0));

    float dt = 0.25;

    if (pc.brush_enabled == 1 && distance(location, brush_pos) < dist) {
        imageStore(img, location, vec4(1.0, 0.0, 0.0, 1.0));
    } else {
        float v = V(location);
        float du_dt = du_dt(location);
        float new_v = v + du_dt * dt;

        float u = U(location);
        float new_u = u + new_v * dt;

        imageStore(img, location, vec4(new_u, new_v, 0.0, 0.0));
    }

}
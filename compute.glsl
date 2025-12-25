#version 460

layout (local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout (push_constant) uniform PushConstantData {
    float time_step;
    int brush_x;
    int brush_y;
    int brush_enabled;
    int brush_radius;
    int brush_value;
    int brush_layer;
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

float potential(int x, int y) {
    if (x < 0 || x >= imageSize(img).x || y < 0 || y >= imageSize(img).y)
        return 0.0;
    return imageLoad(img, ivec2(x, y)).b;
}

float potential(ivec2 location) {
    return potential(location.x, location.y);
}

float du_dt(int x, int y) {
    float dx = 1.0;

    float v = V(x, y);
    float dv_dx_0 = (v - V(x-1, y)) / dx;
    float dv_dx_1 = (V(x+1, y) - v) / dx;

    float dv_dy_0 = (v - V(x, y-1)) / dx;
    float dv_dy_1 = (V(x, y+1) - v) / dx;

    float d2v_dx2 = (dv_dx_1 - dv_dx_0) / dx;
    float d2v_dy2 = (dv_dy_1 - dv_dy_0) / dx;

    return -(d2v_dx2 + d2v_dy2) + potential(x, y) * V(x, y);
}

float du_dt(ivec2 location) {
    return du_dt(location.x, location.y);
}

float dv_dt(int x, int y) {
    float dx = 1.0;

    float u = U(x, y);
    float du_dx_0 = (u - U(x-1, y)) / dx;
    float du_dx_1 = (U(x+1, y) - u) / dx;

    float du_dy_0 = (u - U(x, y-1)) / dx;
    float du_dy_1 = (U(x, y+1) - u) / dx;

    float d2u_dx2 = (du_dx_1 - du_dx_0) / dx;
    float d2u_dy2 = (du_dy_1 - du_dy_0) / dx;

    return (d2u_dx2 + d2u_dy2) - potential(x, y) * V(x, y);
}

float dv_dt(ivec2 location) {
    return dv_dt(location.x, location.y);
}

void main() {
    ivec2 location = ivec2(gl_GlobalInvocationID.xy);
    int x = location.x;
    int y = location.y;

    float dt = pc.time_step;

    ivec2 brush_pos = ivec2(pc.brush_x, pc.brush_y);
    float brush_radius = float(pc.brush_radius);
    float brush_value = float(pc.brush_value);
    float r = distance(location, brush_pos);

    vec4 grid_cell = imageLoad(img, location);
    float u = grid_cell.r;
    float v = grid_cell.g;
    float potential = grid_cell.b;

    if (pc.brush_enabled == 1 && r < brush_radius) {
        if (pc.brush_layer == 0) {
            imageStore(img, location, vec4(max(u, brush_value * exp(- pow(r, 2) / brush_radius)), v, potential, 0.0));
        } else if (pc.brush_layer == 1) {
            imageStore(img, location, vec4(u, max(v, brush_value * exp(- pow(r, 2) / brush_radius)), potential, 0.0));
        } else if (pc.brush_layer == 3) {
            imageStore(img, location, vec4(u, v, max(potential, brush_value * exp(- pow(r, 2) / brush_radius)), 0.0));
        }
    } else {
        float du_dt = du_dt(location);
        float new_u = u + du_dt * dt;

        imageStore(img, location, vec4(new_u, v, potential, 0.0));

        float dv_dt = dv_dt(location);
        float new_v = v + dv_dt * dt;

        imageStore(img, location, vec4(new_u, new_v, potential, 0.0));
    }

}
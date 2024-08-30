fn srgb_to_linear(
    coord: vec4f
) -> vec4f {
    let linear = vec4f(
        pow(coord.r, 2.2),
        pow(coord.g, 2.2),
        pow(coord.b, 2.2),
        coord.a
    );
    return linear;
}
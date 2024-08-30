// MACROS 2
// This tests macros with comments and parenthesis.

#define TO_VEC(r, g, b) vec3f(r, g, b)
#define TO_VEC4(vec3) vec4f(vec3, 1.0f)

fn to_grey(x: f32) -> vec4f {
    return TO_VEC4(TO_VEC(f32(x), /* even, comments, like, this */ f32(x), f32(x)));
}

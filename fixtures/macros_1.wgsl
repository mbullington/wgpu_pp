// MACROS 1
// This tests basic macros.

#define TO_F32(x) f32(x)

fn to_f32(x: f32) -> f32 {
    return TO_F32(x);
}

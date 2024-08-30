// DEFINE 3
// This tests undef.

#define PI f32(3.14159)
#undef PI

fn times_pi(x: f32) -> f32 {
    // Should still be PI.
    return x * PI;
}

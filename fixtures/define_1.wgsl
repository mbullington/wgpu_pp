// DEFINE 1
// Just the basics.

#define PI f32(3.14159)

fn times_pi(x: f32) -> f32 {
    return x * PI;
}

// This shouldn't be converted, since it's not an identifier.
fn times_PI_bad(x: f32) -> f32 {
    return 0.0;
}
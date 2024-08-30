// COMMENTS 2
// Block comments

#define PI (3.14159)

/* this should work */ #include "basic.wgsl" /* this too */

/* this should not work #undef PI */

/* multi line
// */

fn times_pi(x: f32) -> f32 {
    // Should still be PI.
    return x * PI;
}

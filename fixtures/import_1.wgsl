// IMPORT 1
// This tests a cyclic dependency between import_1 and import_2.

#include "import_2.wgsl"

fn import_1(x: f32) -> f32 {
    return 0.0;
}

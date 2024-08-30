// IMPORT 2
// This tests a cyclic dependency between import_1 and import_2.

#include "import_1.wgsl"

fn import_2(x: f32) -> f32 {
    return 0.0;
}

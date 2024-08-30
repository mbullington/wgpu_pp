// DEFINE 2
// Along with basic.wgsl, this shows that #define works across files.
// We're replacing srgb_to_linear with srgb_to_linear2

#define srgb_to_linear srgb_to_linear2

#include "basic.wgsl"
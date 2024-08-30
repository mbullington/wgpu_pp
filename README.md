`wgpu-pp`
===

Simple, C-style preprocessor for WGSL using proc macros. It also runs compile-time validation via Naga.

```rust
include_wgsl!("../shader.wgsl")
```

`shader.wgsl`:

```wgsl
#include "common/color.wgsl"

#define PI f32(3.14159)

@fragment
fn fs_main(
    @builtin(position) position: vec4f,
) -> @location(0) vec4f {
    let x = 2.0 * (position.x - 0.5) * PI
    let y = 2.0 * (position.y - 0.5) * PI

    return srgb_to_linear(vec4(
        0.5 + 0.5 * sin(x),
        0.0,
        0.5 + 0.5 * cos(x),
        1.0
    ));
}
```

Preprocessor statements:

- `#include`: Ability to share code between shaders.
- `#define`: Work around WGSL 1.0 limitations (such as [passing arrays to functions](https://github.com/gpuweb/gpuweb/issues/2268#issuecomment-1788285679)). Works for both constants and macros.

`wgpu-pp` does not aim to output human-readable WGSL, there may be extraneous newlines—comments are also stripped.

## License

This work is distributed under the MIT License.
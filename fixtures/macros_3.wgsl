// MACROS 3
// This tests macros with the \ character for multi line, and function defs.

#define GET_U8(arr, u8_index) \
    u32( \
        u32(arr[u8_index / u32(4)] >> (u32(u8_index % u32(4)) * u32(8))) & u32(0xFF) \
    )

fn get_from_packed_u8_flats(u8_index: u32) -> u32 {
    return GET_U8(flats, u8_index);
}

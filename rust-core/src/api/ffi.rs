// ffi.rs - voorbeeld extern C API
use std::os::raw::c_char;

#[no_mangle]
pub extern \"C\" fn dummy() -> u32 {
    1
}

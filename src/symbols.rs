use crate::rust_string::RustString;

#[no_mangle]
pub extern "C" fn new_rust_string() -> RustString {
    let string = String::new();
    RustString::from_string(string)
}

#[no_mangle]
pub unsafe extern "C" fn rust_string_len(r_string: *const RustString) -> usize {
    (*r_string).len()
}

use std::mem::transmute;

#[repr(C)]
pub struct RustString {
    // A string in Rust is a Vec<u8>, and Vec is three pointer sized fields
    repr: [usize; 3],
}

impl RustString {
    pub fn from_string(string: String) -> RustString {
        unsafe { transmute(string) }
    }

    pub fn from_string_ref(string: &String) -> &RustString {
        unsafe { transmute(string) }
    }

    pub fn from_string_mut(string: &mut String) -> &mut RustString {
        unsafe { transmute(string) }
    }

    pub fn to_string(r_string: RustString) -> String {
        unsafe { transmute(r_string) }
    }

    pub fn as_string_ref(&self) -> &String {
        unsafe { transmute(self) }
    }

    pub fn as_string_mut(&mut self) -> &mut String {
        unsafe { transmute(self) }
    }

    pub fn len(&self) -> usize {
        self.as_string_ref().len()
    }
}

const _: () = assert!(std::mem::size_of::<RustString>() == std::mem::size_of::<String>());
const _: () = assert!(std::mem::align_of::<RustString>() == std::mem::align_of::<String>());

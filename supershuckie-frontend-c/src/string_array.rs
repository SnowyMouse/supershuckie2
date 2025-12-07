use std::ffi::c_char;
use std::ptr::null;
use supershuckie_frontend::util::UTF8CString;

#[repr(transparent)]
#[derive(Default)]
pub struct SuperShuckieStringArray(pub Vec<UTF8CString>);

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_stringarray_len(
    arr: &SuperShuckieStringArray
) -> usize {
    arr.0.len()
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_stringarray_get(
    arr: &SuperShuckieStringArray,
    element: usize
) -> *const c_char {
    arr.0.get(element).map(|i| i.as_c_str().as_ptr()).unwrap_or(null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_stringarray_free(
    arr: *mut SuperShuckieStringArray
) {
    if !arr.is_null() {
        let _ = unsafe { Box::from_raw(arr) };
    }
}

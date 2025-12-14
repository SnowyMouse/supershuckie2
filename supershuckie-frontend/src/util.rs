use std::ffi::{CStr, CString};
use std::str::FromStr;

/// A string that is guaranteed to be UTF-8 while also having its buffer being null-terminated.
#[repr(transparent)]
pub struct UTF8CString {
    inner: CString
}

impl UTF8CString {
    pub fn new<T: Into<Vec<u8>>>(what: T) -> Self {
        Self { inner: CString::new(what).expect("UTF8CString::new failed") }
    }

    pub fn from_str(str: &str) -> Self {
        Self { inner: CString::from_str(str).expect("UTF8CString::from_str failed") }
    }

    pub fn as_str(&self) -> &str {
        self.inner.to_str().unwrap()
    }

    pub fn as_c_str(&self) -> &CStr {
        self.inner.as_c_str()
    }
}

impl AsRef<str> for UTF8CString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for UTF8CString {
    fn from(value: &str) -> Self {
        Self::from_str(value)
    }
}

impl From<String> for UTF8CString {
    fn from(value: String) -> Self {
        let mut bytes = value.into_bytes();
        bytes.push(0);
        let inner = CString::from_vec_with_nul(bytes).expect("UTF8CString::from::<String> fail");
        Self { inner }
    }
}

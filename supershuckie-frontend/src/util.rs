use std::ffi::{CStr, CString};
use std::fmt::Formatter;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

/// A string that is guaranteed to be UTF-8 while also having its buffer being null-terminated.
#[repr(transparent)]
#[derive(Clone, PartialEq, Default)]
pub struct UTF8CString {
    inner: CString
}

impl core::fmt::Display for UTF8CString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for UTF8CString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UTF8CString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_str(UTF8CStringVisitor)
    }
}

struct UTF8CStringVisitor;
impl<'de> Visitor<'de> for UTF8CStringVisitor {
    type Value = UTF8CString;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string (with no nul bytes)")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error
    {
        if v.contains(|i| i == 0u8 as char) {
            return Err(Error::custom("nul char found (not allowed)"))
        }

        Ok(UTF8CString::from_str(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error
    {
        if v.contains(|i| i == 0u8 as char) {
            return Err(Error::custom("nul char found (not allowed)"))
        }

        Ok(UTF8CString::from_str(v.as_str()))
    }
}

impl UTF8CString {
    #[inline]
    pub fn new<T: Into<Vec<u8>>>(what: T) -> Self {
        Self { inner: CString::new(what).expect("UTF8CString::new failed") }
    }

    #[inline]
    pub fn from_str(str: &str) -> Self {
        Self { inner: CString::from_str(str).expect("UTF8CString::from_str failed") }
    }

    #[inline]
    pub fn from_cstr(str: &CStr) -> Self {
        Self { inner: str.to_owned() }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.to_str().unwrap()
    }

    #[inline]
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

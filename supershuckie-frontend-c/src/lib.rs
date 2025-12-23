/// If $what is null, yield a reference to a dummy value. Otherwise, dereference it.
macro_rules! nullable_reference {
    ($what:expr) => {
        if $what.is_null() { &mut core::mem::zeroed() } else { &mut *$what }
    };
}

pub mod frontend;
pub mod string_array;
pub mod control_settings;

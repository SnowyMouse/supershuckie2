use std::borrow::Cow;
use std::ffi::{c_char, CStr};
use std::ptr::null_mut;
use crate::PokeAByteError;

pub struct PokeAByteSharedMemory {
    memory: &'static mut [u8]
}

// TODO: This is not really the correct way to do this. macOS limits mmap to 4 MB for example; we
// probably need to do some sort of repeated call to mmap() to get all "views" of the shared object
// in that case
pub(crate) const POKE_A_BYTE_SHARED_MEMORY_LEN: usize = 1024 * 1024 * 4;

unsafe extern "C" {
    fn supershuckie_pokeabyte_try_create_shared_memory(len: usize, error: *mut *mut c_char) -> *mut u8;
    fn supershuckie_pokeabyte_close_shared_memory();
}

impl PokeAByteSharedMemory {
    pub(crate) fn new() -> Result<PokeAByteSharedMemory, PokeAByteError> {
        let mut error = null_mut();
        let memory = unsafe {
            let ram = supershuckie_pokeabyte_try_create_shared_memory(POKE_A_BYTE_SHARED_MEMORY_LEN, &mut error);
            if ram.is_null() {
                return Err(PokeAByteError::SharedMemoryFailure { explanation: Cow::Owned(format!("Error: {}", CStr::from_ptr(error).to_str().unwrap())) })
            }
            std::slice::from_raw_parts_mut(ram, POKE_A_BYTE_SHARED_MEMORY_LEN)
        };

        Ok(Self {
            memory
        })
    }

    /// # Safety
    ///
    /// There is no protection against data races from other processes.
    #[inline]
    pub unsafe fn get_memory(&self) -> &[u8] {
        self.memory
    }

    /// # Safety
    ///
    /// There is no protection against data races from other processes.
    #[inline]
    pub unsafe fn get_memory_mut(&mut self) -> &mut [u8] {
        self.memory
    }
}

impl Drop for PokeAByteSharedMemory {
    fn drop(&mut self) {
        unsafe { supershuckie_pokeabyte_close_shared_memory() };
    }
}


unsafe impl Sync for PokeAByteSharedMemory {}
unsafe impl Send for PokeAByteSharedMemory {}

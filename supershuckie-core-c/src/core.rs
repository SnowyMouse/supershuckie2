use std::ptr::slice_from_raw_parts_mut;
use std::slice::from_raw_parts;
use supershuckie_core::emulator::{GameBoyColor, Input, Model, NullEmulatorCore};
use supershuckie_core::ThreadedSuperShuckieCore;

#[repr(transparent)]
pub struct SuperShuckieCoreC {
    core: ThreadedSuperShuckieCore
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_new_null() -> *mut SuperShuckieCoreC {
    let core = SuperShuckieCoreC {
        core: ThreadedSuperShuckieCore::new(Box::new(NullEmulatorCore))
    };

    Box::into_raw(Box::new(core))
}

#[repr(C)]
pub enum GBCType {
    GB,
    GBC
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_core_new_gameboy(
    rom_data: *const u8,
    rom_len: usize,
    bios_data: *const u8,
    bios_len: usize,
    gb_type: GBCType
) -> *mut SuperShuckieCoreC {
    debug_assert!(!rom_data.is_null());
    debug_assert!(!bios_data.is_null());

    let rom_data = unsafe { from_raw_parts(rom_data, rom_len) };
    let bios_data = unsafe { from_raw_parts(bios_data, bios_len) };

    let model = match gb_type {
        GBCType::GB => Model::DmgB,
        GBCType::GBC => Model::Cgb0
    };

    let gbc = Box::new(GameBoyColor::new_from_rom(
        rom_data, bios_data, model
    ));

    let core = SuperShuckieCoreC {
        core: ThreadedSuperShuckieCore::new(gbc)
    };

    Box::into_raw(Box::new(core))
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_get_frame_count(core: &SuperShuckieCoreC) -> u32 {
    core.core.get_frame_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_get_screen_count(core: &SuperShuckieCoreC) -> usize {
    core.core.read_screens(|f| f.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_start(core: &mut SuperShuckieCoreC) {
    core.core.start()
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_pause(core: &mut SuperShuckieCoreC) {
    core.core.pause()
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_enqueue_input(core: &mut SuperShuckieCoreC, input: &InputC) {
    core.core.enqueue_input((*input).into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_core_free(raw: *mut SuperShuckieCoreC) {
    if !raw.is_null() {
        let _ = unsafe { Box::from_raw(raw) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_core_get_screen_resolution(core: &mut SuperShuckieCoreC, screen_index: usize, width: &mut usize, height: &mut usize) -> bool {
    core.core.read_screens(|s| {
        let Some(screen) = s.get(screen_index) else {
            return false
        };

        *width = screen.width;
        *height = screen.height;

        true
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_core_copy_screen_data(core: &SuperShuckieCoreC, screen_index: usize, data: *mut u32, data_size: usize) -> usize {
    let data_slice = slice_from_raw_parts_mut(data, data_size);

    core.core.read_screens(|s| {
        let Some(screen) = s.get(screen_index) else {
            return 0
        };

        let width = screen.width;
        let height = screen.height;

        let pixel_count = width * height;
        if data_size != 0 {
            let data_slice = unsafe { &mut *data_slice };
            let copyable_pixel_count = data_slice.len().min(pixel_count);
            data_slice[..copyable_pixel_count].copy_from_slice(&screen.pixels[..copyable_pixel_count]);
        }

        pixel_count
    })
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct InputC {
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,

    pub d_up: bool,
    pub d_down: bool,
    pub d_left: bool,
    pub d_right: bool,

    pub l: bool,
    pub r: bool,
    pub x: bool,
    pub y: bool,

    pub touch_x: u16,
    pub touch_y: u16
}

impl From<InputC> for Input {
    fn from(value: InputC) -> Self {
        Self {
            a: value.a,
            b: value.b,
            start: value.start,
            select: value.select,
            d_up: value.d_up,
            d_down: value.d_down,
            d_left: value.d_left,
            d_right: value.d_right,
            l: value.l,
            r: value.r,
            x: value.x,
            y: value.y,
            touch: if value.touch_x == u16::MAX || value.touch_y == u16::MAX { None } else { Some((value.touch_x, value.touch_y)) },
        }
    }
}

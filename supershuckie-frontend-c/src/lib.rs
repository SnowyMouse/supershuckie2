use std::ffi::{c_char, c_void, CStr};
use std::mem::MaybeUninit;
use std::ptr::null;
use std::slice::from_raw_parts_mut;
use supershuckie_core::emulator::{Input, ScreenData, ScreenDataEncoding};
use supershuckie_frontend::{CoreMetadata, SuperShuckieFrontend, SuperShuckieFrontendCallbacks};
use supershuckie_frontend::settings::ControlSetting;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SuperShuckieScreenDataC {
    pub width: u32,
    pub height: u32,
    pub screen_data_encoding: ScreenDataEncoding
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SuperShuckieFrontendCallbacksC {
    pub userdata: *mut c_void,

    pub refresh_screens: Option<unsafe extern "C" fn(userdata: *mut c_void, screen_count: usize, screen_data: *const *const u32)>,
    pub new_core_metadata: Option<unsafe extern "C" fn(userdata: *mut c_void, screen_count: usize, screen_data: *const SuperShuckieScreenDataC)>,
}

impl SuperShuckieFrontendCallbacks for SuperShuckieFrontendCallbacksC {
    fn refresh_screens(&mut self, screens: &[ScreenData]) {
        let Some(s) = self.refresh_screens else { return };

        let mut screens_buf = [null(); 4];
        for (index, screen) in screens.iter().enumerate() {
            screens_buf[index] = screen.pixels.as_ptr();
        }

        unsafe { s(self.userdata, screens.len(), screens_buf.as_ptr()) };
    }

    fn new_core_metadata(&mut self, _core_metadata: &CoreMetadata, screens: &[ScreenData]) {
        let Some(s) = self.new_core_metadata else { return };

        let mut screens_buf = [MaybeUninit::<SuperShuckieScreenDataC>::uninit(); 4];
        for (index, screen) in screens.iter().enumerate() {
            screens_buf[index].write(SuperShuckieScreenDataC {
                width: screen.width as u32,
                height: screen.height as u32,
                screen_data_encoding: screen.encoding
            });
        }

        unsafe { s(self.userdata, screens.len(), screens_buf.as_ptr() as *const SuperShuckieScreenDataC) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_new(
    user_dir: *const c_char,
    callbacks: &SuperShuckieFrontendCallbacksC
) -> *mut SuperShuckieFrontend {
    let user_dir = unsafe { CStr::from_ptr(user_dir) }
        .to_str()
        .expect("path is not UTF-8");

    Box::into_raw(Box::new(SuperShuckieFrontend::new(user_dir, Box::new(*callbacks))))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_key_press(
    frontend: &mut SuperShuckieFrontend,
    key_code: u8,
    pressed: bool
) {
    if let Some(&s) = frontend.get_settings().keyboard_controls.mappings.get(&key_code) {
        frontend.set_button_input(&s, pressed);
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_set_paused(
    frontend: &mut SuperShuckieFrontend,
    paused: bool
) {
    frontend.set_paused(paused);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_tick(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.tick();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_load_rom(
    frontend: &mut SuperShuckieFrontend,
    path: *const c_char,
    error: *mut u8,
    error_len: usize
) -> bool {
    let path = unsafe { CStr::from_ptr(path) };
    if let Err(e) = frontend.load_rom(path.to_str().expect("supershuckie_frontend_load_rom with non-UTF-8 path")) {
        write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
        false
    }
    else {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_unload_rom(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.unload_rom();
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_frontend_is_game_running(
    frontend: &SuperShuckieFrontend
) -> bool {
    frontend.is_game_running()
}

fn write_str_to_data(string: &str, buffer: &mut [u8]) {
    if buffer.is_empty() {
        return
    }
    buffer.fill(0);

    let buffer_length = buffer.len();
    let mut buffer_usable = &mut buffer[0..buffer_length - 1]; // need the last byte to be null terminated
    if buffer_usable.is_empty() {
        return
    }

    let mut char_data = [0u8; 4];
    for c in string.chars() {
        let bytes = c.encode_utf8(&mut char_data).as_bytes();
        let Some((a, b)) = buffer_usable.split_at_mut_checked(bytes.len()) else {
            return
        };
        a.copy_from_slice(bytes);
        buffer_usable = b;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_force_refresh_screens(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.force_refresh_screens();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_rom_name(
    frontend: &SuperShuckieFrontend
) -> *const c_char {
    frontend.get_current_rom_name_c_str().map(|i| i.as_ptr()).unwrap_or(null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_write_settings(
    frontend: &SuperShuckieFrontend
) {
    frontend.write_settings();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_keyboard_control_setting(
    frontend: &SuperShuckieFrontend,
    key_code: u8,
    setting: *mut ControlSetting
) -> bool {
    let Some(s) = frontend.get_settings().keyboard_controls.mappings.get(&key_code) else {
        return false
    };

    if !setting.is_null() {
        unsafe { *setting = *s };
    }

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_free(
    frontend: *mut SuperShuckieFrontend
) {
    if !frontend.is_null() {
        let _ = unsafe { Box::from_raw(frontend) };
    }
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

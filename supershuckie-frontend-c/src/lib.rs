use std::ffi::{c_char, c_void, CStr};
use std::mem::MaybeUninit;
use std::num::NonZeroU8;
use std::ptr::null;
use std::slice::from_raw_parts_mut;
use supershuckie_core::emulator::{Input, ScreenData, ScreenDataEncoding};
use supershuckie_frontend::{SuperShuckieFrontend, SuperShuckieFrontendCallbacks};
use supershuckie_frontend::settings::ControlSetting;
use supershuckie_frontend::util::UTF8CString;

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
    pub change_video_mode: Option<unsafe extern "C" fn(userdata: *mut c_void, screen_count: usize, screen_data: *const SuperShuckieScreenDataC, screen_scale: NonZeroU8)>,
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

    fn change_video_mode(&mut self, screens: &[ScreenData], scaling: NonZeroU8) {
        let Some(s) = self.change_video_mode else { return };

        let mut screens_buf = [MaybeUninit::<SuperShuckieScreenDataC>::uninit(); 4];
        for (index, screen) in screens.iter().enumerate() {
            screens_buf[index].write(SuperShuckieScreenDataC {
                width: screen.width as u32,
                height: screen.height as u32,
                screen_data_encoding: screen.encoding
            });
        }

        unsafe { s(self.userdata, screens.len(), screens_buf.as_ptr() as *const SuperShuckieScreenDataC, scaling) };
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
    if error_len > 0 && let Err(e) = frontend.load_rom(path.to_str().expect("supershuckie_frontend_load_rom with non-UTF-8 path")) {
        write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
        false
    }
    else {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_close_rom(
    frontend: &mut SuperShuckieFrontend
) {
    let _ = frontend.close_rom();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_unload_rom(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.unload_rom();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_load_or_create_save_file(
    frontend: &mut SuperShuckieFrontend,
    save_file: *const c_char,
    initialize: bool
) {
    let save_file = unsafe { CStr::from_ptr(save_file) }.to_str().expect("save file not utf-8");
    frontend.load_or_create_save_file(save_file, initialize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_set_current_save_file(
    frontend: &mut SuperShuckieFrontend,
    save_file: *const c_char
) {
    let save_file = unsafe { CStr::from_ptr(save_file) }.to_str().expect("save file not utf-8");
    frontend.set_current_save_file(save_file);
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_frontend_hard_reset_console(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.hard_reset_console();
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
pub unsafe extern "C" fn supershuckie_frontend_set_video_scale(
    frontend: &mut SuperShuckieFrontend,
    scale: u8
) {
    frontend.set_video_scale(NonZeroU8::new(scale).unwrap_or(unsafe { NonZeroU8::new_unchecked(1) }));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_custom_setting(
    frontend: &SuperShuckieFrontend,
    setting: *const c_char
) -> *const c_char {
    frontend.get_custom_setting(unsafe { CStr::from_ptr(setting) }.to_str().expect("supershuckie_frontend_get_custom_setting bad setting"))
        .map(|i| i.as_c_str().as_ptr())
        .unwrap_or(null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_start_recording_replay(
    frontend: &mut SuperShuckieFrontend,
    name: *const c_char,
    result: *mut u8,
    result_len: usize
) -> bool {
    let name = if !name.is_null() { Some(unsafe { CStr::from_ptr(name) }.to_str().expect("name not UTF-8")) } else { None };
    let (success, msg) = match frontend.start_recording_replay(name) {
        Ok(n) => (true, n),
        Err(n) => (false, n)
    };

    write_str_to_data(msg.as_str(), unsafe { from_raw_parts_mut(result, result_len) });
    success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_stop_recording_replay(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.stop_recording_replay();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_recording_replay_file(
    frontend: &SuperShuckieFrontend
) -> *const c_char {
    frontend.get_replay_file_info().map(|i| i.final_replay_name.as_c_str().as_ptr()).unwrap_or(null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_recording_replay_milliseconds(
    frontend: &SuperShuckieFrontend
) -> u32 {
    frontend.get_recorded_replay_milliseconds()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_create_save_state(
    frontend: &mut SuperShuckieFrontend,
    name: *const c_char,
    result: *mut u8,
    result_len: usize
) -> bool {
    let name = if !name.is_null() { Some(unsafe { CStr::from_ptr(name) }.to_str().expect("name not UTF-8")) } else { None };
    let (success, msg) = match frontend.create_save_state(name) {
        Ok(n) => (true, n),
        Err(n) => (false, n)
    };

    write_str_to_data(msg.as_str(), unsafe { from_raw_parts_mut(result, result_len) });
    success
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_undo_load_save_state(
    frontend: &mut SuperShuckieFrontend
) -> bool {
    frontend.undo_load_save_state()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_redo_load_save_state(
    frontend: &mut SuperShuckieFrontend
) -> bool {
    frontend.redo_load_save_state()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_load_save_state(
    frontend: &mut SuperShuckieFrontend,
    name: *const c_char,
    error: *mut u8,
    error_len: usize
) -> bool {
    let name = unsafe { CStr::from_ptr(name) }.to_str().expect("name not UTF-8");
    match frontend.load_save_state_if_exists(name) {
        Ok(true) => true,
        Ok(false) => {
            if error_len >= 1 {
                unsafe { *error = 0 };
            }
            false
        }
        Err(_) if error_len == 0 => false,
        Err(e) => {
            write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_is_pokeabyte_enabled(
    frontend: &mut SuperShuckieFrontend,
    error: *mut u8,
    error_len: usize
) -> bool {
    match frontend.is_pokeabyte_enabled() {
        Ok(n) => {
            unsafe { *error = 0 };
            n
        },
        Err(e) => {
            write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_set_pokeabyte_enabled(
    frontend: &mut SuperShuckieFrontend,
    enabled: bool,
    error: *mut u8,
    error_len: usize
) -> bool {
    match frontend.set_pokeabyte_enabled(enabled) {
        Ok(_) => true,
        Err(e) => {
            write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_save_sram(
    frontend: &mut SuperShuckieFrontend,
    error: *mut u8,
    error_len: usize
) -> bool {
    match frontend.save_sram() {
        Ok(_) => true,
        Err(_) if error_len == 0 => false,
        Err(e) => {
            write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_set_custom_setting(
    frontend: &mut SuperShuckieFrontend,
    setting: *const c_char,
    value: *const c_char
) {
    frontend.set_custom_setting(
        unsafe { CStr::from_ptr(setting) }.to_str().expect("supershuckie_frontend_set_custom_setting bad setting"),
        if value.is_null() {
            None
        }
        else {
            Some(UTF8CString::from_cstr(unsafe { CStr::from_ptr(value) }))
        }
    );
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
pub unsafe extern "C" fn supershuckie_frontend_get_speed_settings(
    frontend: &SuperShuckieFrontend,
    base: *mut f64,
    turbo: *mut f64
) {
    let emulation_settings = &frontend.get_settings().emulation;
    if !base.is_null() {
        unsafe { *base = emulation_settings.base_speed_multiplier };
    }
    if !turbo.is_null() {
        unsafe { *turbo = emulation_settings.turbo_speed_multiplier };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn supershuckie_frontend_set_speed_settings(
    frontend: &mut SuperShuckieFrontend,
    base: f64,
    turbo: f64
) {
    frontend.set_speed(base, turbo);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_free(
    frontend: *mut SuperShuckieFrontend
) {
    if !frontend.is_null() {
        let _ = unsafe { Box::from_raw(frontend) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_all_replays_for_rom(
    frontend: &SuperShuckieFrontend,
    rom: *const c_char
) -> *mut SuperShuckieStringArray {
    let array = match unsafe { current_rom_or_null(frontend, rom) } {
        Some(rom) => SuperShuckieStringArray(frontend.get_all_replays_for_rom(rom)),
        None => SuperShuckieStringArray::default()
    };
    Box::into_raw(Box::new(array))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_all_saves_for_rom(
    frontend: &SuperShuckieFrontend,
    rom: *const c_char
) -> *mut SuperShuckieStringArray {
    let array = match unsafe { current_rom_or_null(frontend, rom) } {
        Some(rom) => SuperShuckieStringArray(frontend.get_all_saves_for_rom(rom)),
        None => SuperShuckieStringArray::default()
    };
    Box::into_raw(Box::new(array))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_all_save_states_for_rom(
    frontend: &SuperShuckieFrontend,
    rom: *const c_char
) -> *mut SuperShuckieStringArray {
    let array = match unsafe { current_rom_or_null(frontend, rom) } {
        Some(rom) => SuperShuckieStringArray(frontend.get_all_save_states_for_rom(rom)),
        None => SuperShuckieStringArray::default()
    };
    Box::into_raw(Box::new(array))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_get_replay_playback_stats(
    frontend: &SuperShuckieFrontend,
    total_frames: *mut u32,
    total_milliseconds: *mut u32
) -> bool {
    let total_frames = unsafe { if total_frames.is_null() { &mut 0 } else { &mut *total_frames } };
    let total_milliseconds = unsafe { if total_milliseconds.is_null() { &mut 0 } else { &mut *total_milliseconds } };

    match frontend.get_replay_playback_stats() {
        Some(n) => {
            *total_frames = n.total_frames;
            *total_milliseconds = n.total_milliseconds;
            true
        },
        None => {
            *total_frames = 0;
            *total_milliseconds = 0;
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_load_replay(
    frontend: &mut SuperShuckieFrontend,
    name: *const c_char,
    override_errors: bool,
    error: *mut u8,
    error_len: usize
) -> bool {
    let name = unsafe { CStr::from_ptr(name).to_str().expect("replay name is not UTF-8") };

    match frontend.load_replay_if_exists(name, override_errors) {
        Ok(_) => true,
        Err(e) => {
            write_str_to_data(e.as_str(), unsafe { from_raw_parts_mut(error, error_len) });
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_frontend_stop_replay_playback(
    frontend: &mut SuperShuckieFrontend
) {
    frontend.stop_replay_playback();
}

unsafe fn current_rom_or_null(frontend: &SuperShuckieFrontend, rom: *const c_char) -> Option<&str> {
    if rom.is_null() {
        frontend.get_current_rom_name()
    }
    else {
        Some(unsafe { CStr::from_ptr(rom) }.to_str().expect("save file not utf-8"))
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

#[repr(transparent)]
#[derive(Default)]
pub struct SuperShuckieStringArray(Vec<UTF8CString>);

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

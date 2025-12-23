use supershuckie_frontend::settings::Controls;

pub struct SuperShuckieControlSettings(pub Controls);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_control_settings_get_keyboard_key(
    settings: &SuperShuckieControlSettings
) {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supershuckie_control_settings_free(
    settings: *mut SuperShuckieControlSettings
) {
    if !settings.is_null() {
        let _ = unsafe { Box::from_raw(settings) };
    }
}

use std::ffi::CString;
use std::ptr::{null, null_mut};
use std::str::FromStr;
use sdl3_sys::everything::{SDL_MessageBoxButtonData, SDL_MessageBoxData, SDL_ShowMessageBox, SDL_MESSAGEBOX_ERROR, SDL_MESSAGEBOX_INFORMATION, SDL_MESSAGEBOX_WARNING};

#[allow(unused)]
pub enum MessageType {
    Error,
    Warning,
    Information
}

pub fn show_message(message_type: MessageType, title: &str, message: &str) {
    let title = CString::from_str(title).expect("failed to make title cstr");
    let message = CString::from_str(message).expect("failed to make message cstr");

    let button = SDL_MessageBoxButtonData {
        flags: 0,
        buttonID: 0,
        text: c"OK".as_ptr(),
    };

    let message_box_data = SDL_MessageBoxData {
        flags: match message_type {
            MessageType::Error => SDL_MESSAGEBOX_ERROR,
            MessageType::Warning => SDL_MESSAGEBOX_WARNING,
            MessageType::Information => SDL_MESSAGEBOX_INFORMATION
        },
        window: null_mut(),
        title: title.as_ptr(),
        message: message.as_ptr(),
        numbuttons: 1,
        buttons: &button,
        colorScheme: null(),
    };

    unsafe { SDL_ShowMessageBox(&message_box_data, null_mut()) };
}

mod renderer;

use sdl3_main::{app_impl, AppResult};
use sdl3_sys::events::{SDL_Event, SDL_EventType};
use std::sync::Mutex;
use sdl3_sys::everything::SDL_WINDOW_VULKAN;
use sdl3_sys::video::SDL_CreateWindow;

#[derive(Default)]
struct SuperShuckie {
}

#[app_impl]
impl SuperShuckie {
    fn app_init() -> Option<Box<Mutex<SuperShuckie>>> {
        let window = unsafe {
            SDL_CreateWindow(
                c"Super Shuckie 2.0 (name TBD)".as_ptr(),
                640,
                480,
                SDL_WINDOW_VULKAN
            )
        };

        if window.is_null() {
            return None
        }

        Some(Box::new(Mutex::new(SuperShuckie::default())))
    }

    fn app_iterate(&mut self) -> AppResult {

        AppResult::Continue
    }

    fn app_event(&mut self, event: &SDL_Event) -> AppResult {
        use sdl3_sys::events;

        // SAFETY: this is shared with all events
        let event_type = SDL_EventType(unsafe { event.r#type });

        match event_type {
            events::SDL_EVENT_QUIT => {
                println!("Closing! TODO: (save here maybe?)");
                return AppResult::Success;
            }
            _ => {}
        }

        AppResult::Continue
    }
}

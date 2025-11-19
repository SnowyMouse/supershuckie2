use sdl3_main::{app_impl, AppResult};
use sdl3_sys::events::{SDL_Event, SDL_EventType};
use std::sync::Mutex;

#[derive(Default)]
struct SuperShuckie {
}

#[app_impl]
impl SuperShuckie {
    fn app_init() -> Option<Box<Mutex<SuperShuckie>>> {
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

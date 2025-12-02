mod renderer;
mod error;

use std::ptr::null;
use sdl3_main::{app_impl, AppResult};
use sdl3_sys::events::{SDL_Event, SDL_EventType};
use std::sync::Mutex;
use sdl3_sys::everything::{SDL_MessageBoxData, SDL_ShowMessageBox, SDL_Vulkan_LoadLibrary, SDL_INIT_VIDEO, SDL_WINDOW_VULKAN};
use sdl3_sys::init::{SDL_Init, SDL_INIT_GAMEPAD};
use sdl3_sys::video::SDL_CreateWindow;
use sdl3_sys::vulkan::SDL_Vulkan_GetInstanceExtensions;
use vulkano::VulkanLibrary;
use error::show_message;
use crate::error::MessageType;
use crate::renderer::{preload_vulkan, Renderer};

#[derive(Default)]
struct SuperShuckie {
}

#[app_impl]
impl SuperShuckie {
    fn app_init() -> Option<Box<Mutex<SuperShuckie>>> {
        unsafe {
            SDL_Init(SDL_INIT_VIDEO | SDL_INIT_GAMEPAD);
        }

        if let Err(e) = preload_vulkan() {
            show_message(MessageType::Error, "Failed to load Vulkan", &format!("The raw Vulkan library could not be loaded:\n\n{e}"));
            return None;
        }

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

        let renderer = match unsafe { Renderer::attach_to_sdl(window) } {
            Ok(n) => n,
            Err(e) => {
                show_message(MessageType::Error, "Failed to instantiate renderer", &format!("The renderer could not be instantiated:\n\n{e}"));
                return None;
            }
        };

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

use crate::emulator::{EmulatorCore, Input, RunTime, ScreenData, ScreenDataEncoding};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};
use safeboy::rgb_encoder::encode_a8r8g8b8;
use safeboy::{DirectAccessRegion, Gameboy, GameboyCallbacks, InputButton, RunnableInstanceFunctions, RunningGameboy, TurboMode, VBlankType};
pub use safeboy::Model;

/// Game Boy and Game Boy Color emulator.
///
/// Uses [SameBoy](https://sameboy.github.io) as the underlying core.
pub struct GameBoyColor {
    core: Gameboy,
    turbo_mode: TurboMode,
    callback_data: Arc<GameBoyCallbackData>
}

struct GameBoyCallbackData {
    run_frames: AtomicU32,
    screen: UnsafeCell<ScreenData>
}

unsafe impl Send for GameBoyCallbackData {}
unsafe impl Sync for GameBoyCallbackData {}

impl GameBoyColor {
    /// Instantiate a `GameBoyColor` emulator from the given ROM.
    pub fn new_from_rom(
        rom: &[u8],
        bios: &[u8],
        model: Model
    ) -> Self {
        let mut core = Gameboy::new(model);
        core.load_boot_rom(bios);
        core.load_rom(rom);
        core.set_rgb_encoder(encode_a8r8g8b8);
        core.set_rendering_enabled(true);

        let dimensions = core.get_pixel_buffer();
        let screen_data = ScreenData {
            pixels: dimensions.pixels.to_owned(),
            width: dimensions.width as usize,
            height: dimensions.height as usize,
            encoding: ScreenDataEncoding::A8R8G8B8
        };

        let callback_data = Arc::new(GameBoyCallbackData {
            run_frames: AtomicU32::new(0),
            screen: UnsafeCell::new(screen_data)
        });

        core.set_callbacks(Some(Box::new(CallbackHandler { callback_data: callback_data.clone() })));

        Self {
            turbo_mode: TurboMode::Disabled,
            callback_data,
            core
        }
    }
}

struct CallbackHandler {
    callback_data: Arc<GameBoyCallbackData>
}

impl GameboyCallbacks for CallbackHandler {
    fn vblank(&mut self, instance: &mut RunningGameboy, _vblank_type: VBlankType) {
        // SAFETY: Nothing else can currently access this Arc since GameBoyColor is currently
        //         mutably borrowed.
        let screen = unsafe { &mut *self.callback_data.screen.get() };

        screen.pixels.copy_from_slice(instance.get_pixel_buffer_pixels());
        self.callback_data.run_frames.fetch_add(1, Ordering::Relaxed);
    }
}

/// Returns the region and offset.
fn pokeabyte_protocol_region_from_address(address: u32) -> Option<(DirectAccessRegion, usize)> {
    match address {
        // VRAM
        0x8000..=0x9FFF => Some((DirectAccessRegion::VRAM, address as usize - 0x8000)),

        // WRAM bank #0
        0xC000..=0xDFFF => Some((DirectAccessRegion::RAM, address as usize - 0xC000)),

        // WRAM bank #1 (not the actual address)
        0x10000..=0x11FFF => Some((DirectAccessRegion::RAM, address as usize - 0x10000 + 0x2000)),

        // HRAM
        0xFF80..=0xFFFE => Some((DirectAccessRegion::HRAM, address as usize - 0xFF80)),

        _ => None
    }
}

impl EmulatorCore for GameBoyColor {
    fn run(&mut self) -> RunTime {
        let ticks = self.core.run() as u64;
        let frames = self.callback_data.run_frames.swap(0, Ordering::Relaxed) as u64;
        RunTime { ticks, frames }
    }

    fn run_unlocked(&mut self) -> RunTime {
        self.core.set_turbo_mode(TurboMode::Enabled);
        let timing = self.run();
        self.core.set_turbo_mode(self.turbo_mode);
        timing
    }

    fn read_ram(&self, address: u32, into: &mut [u8]) -> Result<(), &'static str> {
        let Some((region, offset)) = pokeabyte_protocol_region_from_address(address) else {
            return Err("invalid or unknown address");
        };
        let Some(offset_end) = offset.checked_add(into.len()) else {
            return Err("invalid length");
        };

        let region = self.core.direct_access(region);
        let Some(data) = region.data.get(offset..offset_end) else {
            return Err("address+length overflows");
        };
        into.copy_from_slice(data);
        Ok(())
    }

    fn write_ram(&mut self, address: u32, from: &[u8]) -> Result<(), &'static str> {
        let Some((region, offset)) = pokeabyte_protocol_region_from_address(address) else {
            return Err("invalid or unknown address");
        };
        let Some(offset_end) = offset.checked_add(from.len()) else {
            return Err("invalid length");
        };
        let region = self.core.direct_access_mut(region);
        let Some(data) = region.data.get_mut(offset..offset_end) else {
            return Err("address+length overflows");
        };
        data.copy_from_slice(from);
        Ok(())
    }

    #[inline]
    fn ticks_per_second(&self) -> f64 {
        (8 * 1024 * 1024) as f64
    }

    #[inline]
    fn set_speed(&mut self, speed: f64) {
        self.core.set_clock_multiplier(speed);
    }

    fn save_sram(&self) -> Vec<u8> {
        self.core.save_sram()
    }

    fn load_sram(&mut self, state: &[u8]) -> Result<(), String> {
        self.core.load_sram(state);
        Ok(())
    }

    fn create_save_state(&self) -> Vec<u8> {
        self.core.create_save_state()
    }

    fn load_save_state(&mut self, state: &[u8]) -> Result<(), String> {
        self.core.load_save_state(state).map_err(|e| format!("{e:?}"))
    }

    fn encode_input(&self, input: Input, into: &mut Vec<u8>) {
        let mask = (input.a as u8) << InputButton::A
            | (input.b as u8) << InputButton::B
            | (input.start as u8) << InputButton::Start
            | (input.select as u8) << InputButton::Select
            | (input.d_up as u8) << InputButton::Up
            | (input.d_down as u8) << InputButton::Down
            | (input.d_left as u8) << InputButton::Left
            | (input.d_right as u8) << InputButton::Right;
        into.push(mask);
    }

    #[inline]
    fn set_input_encoded(&mut self, input: &[u8]) {
        debug_assert!(input.len() == 1, "set_input_encoded with wrong number of bytes {}", input.len());
        self.core.set_input_button_mask(input[0]);
    }

    #[inline]
    fn get_screens(&self) -> &[ScreenData] {
        // SAFETY: This is going to return a reference with the same lifetime as `self`, thus once
        //         we have to mutably borrow again, the borrow will end.
        let screen_data = unsafe { &*self.callback_data.screen.get() };
        core::slice::from_ref(screen_data)
    }

    #[inline]
    fn hard_reset(&mut self) {
        self.core.reset();
    }
}

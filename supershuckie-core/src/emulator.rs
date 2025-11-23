mod game_boy_color;

use alloc::string::String;
pub use game_boy_color::*;

use alloc::vec::Vec;

/// Emulator core functionality.
pub trait EmulatorCore {
    /// Run the smallest amount of time.
    fn run(&mut self) -> RunTime;

    /// Run the smallest amount of time without any timing.
    fn run_unlocked(&mut self) -> RunTime;

    /// Read RAM at the given address to the given data buffer.
    ///
    /// Note: The way `address` is interpreted is core-specific.
    fn read_ram(&self, address: u32, into: &mut [u8]) -> Result<(), &'static str>;

    /// Write RAM to the given address from the given data buffer.
    ///
    /// Note: The way `address` is interpreted is core-specific.
    fn write_ram(&mut self, address: u32, from: &[u8]) -> Result<(), &'static str>;

    /// Return the number of ticks per second.
    ///
    /// Note: This is not allowed to change.
    fn ticks_per_second(&self) -> f64;

    /// Set the game speed multiplier.
    fn set_speed(&mut self, speed: f64);

    /// Create SRAM.
    fn save_sram(&self) -> Vec<u8>;

    /// Load the given SRAM.
    fn load_sram(&mut self, state: &[u8]) -> Result<(), String>;

    /// Create a save state.
    fn create_save_state(&self) -> Vec<u8>;

    /// Load a save state.
    fn load_save_state(&mut self, state: &[u8]) -> Result<(), String>;

    /// Encode the input.
    ///
    /// `into` can be assumed to be empty
    fn encode_input(&self, input: Input, into: &mut Vec<u8>);

    /// Set the current input.
    ///
    /// It must be encoded by `encode_input`.
    fn set_input_encoded(&mut self, input: &[u8]);

    /// Get the screen(s).
    fn get_screens(&self) -> &[ScreenData];
}

/// Amount of time passed when running the emulator core.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct RunTime {
    /// Emulator ticks passed.
    pub ticks: u64,

    /// Frames passed.
    pub frames: u64
}

/// Describes a current input state.
#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Input {
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

    pub touch: Option<(u16, u16)>
}

/// Describes screen data.
#[derive(Clone, PartialEq)]
pub struct ScreenData {
    /// Pixels, encoded.
    pub pixels: Vec<u32>,

    /// Width in pixels.
    ///
    /// Note: This is not allowed to change.
    pub width: usize,

    /// Height in pixels.
    ///
    /// Note: This is not allowed to change.
    pub height: usize,

    /// Encoding to use.
    ///
    /// Note: This is not allowed to change.
    pub encoding: ScreenDataEncoding
}

/// Describes the color encoding.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ScreenDataEncoding {
    /// 0xAARRGGBB
    A8R8G8B8
}

fn _ensure_emulator_core_is_dyn_compatible(_core: &dyn EmulatorCore) {}

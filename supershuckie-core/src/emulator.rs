//! Functionality for emulator cores.

mod game_boy_color;
mod null;

use alloc::string::String;
pub use game_boy_color::*;
pub use null::*;

use alloc::vec::Vec;

/// Emulator core functionality.
pub trait EmulatorCore: Send + 'static {
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

    /// Hard reset the console.
    ///
    /// This simulates instantly turning it off and on.
    fn hard_reset(&mut self);
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
#[derive(Copy, Clone, PartialEq, Debug, Default)]
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

impl core::ops::BitOr<Input> for Input {
    type Output = Input;
    fn bitor(self, rhs: Input) -> Self::Output {
        Input {
            a: self.a | rhs.a,
            b: self.b | rhs.b,
            start: self.start | rhs.start,
            select: self.select | rhs.select,
            d_up: self.d_up | rhs.d_up,
            d_down: self.d_down | rhs.d_down,
            d_left: self.d_left | rhs.d_left,
            d_right: self.d_right | rhs.d_right,
            l: self.l | rhs.l,
            r: self.r | rhs.r,
            x: self.x | rhs.x,
            y: self.y | rhs.y,
            touch: self.touch.or(rhs.touch),
        }
    }
}

impl core::ops::BitAnd<Input> for Input {
    type Output = Input;
    fn bitand(self, rhs: Input) -> Self::Output {
        Input {
            a: self.a & rhs.a,
            b: self.b & rhs.b,
            start: self.start & rhs.start,
            select: self.select & rhs.select,
            d_up: self.d_up & rhs.d_up,
            d_down: self.d_down & rhs.d_down,
            d_left: self.d_left & rhs.d_left,
            d_right: self.d_right & rhs.d_right,
            l: self.l & rhs.l,
            r: self.r & rhs.r,
            x: self.x & rhs.x,
            y: self.y & rhs.y,
            touch: self.touch.and(rhs.touch),
        }
    }
}

impl core::ops::Not for Input {
    type Output = Input;

    fn not(self) -> Self::Output {
        Self {
            a: !self.a,
            b: !self.b,
            start: !self.start,
            select: !self.select,
            d_up: !self.d_up,
            d_down: !self.d_down,
            d_left: !self.d_left,
            d_right: !self.d_right,
            l: !self.l,
            r: !self.r,
            x: !self.x,
            y: !self.y,
            touch: None
        }
    }
}

impl core::ops::BitAndAssign for Input {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl core::ops::BitOrAssign for Input {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
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

impl Default for ScreenData {
    fn default() -> Self {
        Self {
            pixels: Vec::new(),
            width: 0,
            height: 0,
            encoding: ScreenDataEncoding::A8R8G8B8
        }
    }
}

/// Describes the color encoding.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ScreenDataEncoding {
    /// 0xAARRGGBB
    A8R8G8B8
}

fn _ensure_emulator_core_is_dyn_compatible(_core: &dyn EmulatorCore) {}

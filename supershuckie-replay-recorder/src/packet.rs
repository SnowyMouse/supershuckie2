use tinyvec::TinyVec;
use alloc::string::String;
use alloc::vec::Vec;

mod io;
pub use io::*;

#[allow(missing_docs)]
pub type InputBuffer = TinyVec<[u8; 16]>;
#[allow(missing_docs)]
pub type UnsignedInteger = u64;
#[allow(missing_docs)]
pub type ByteVec = TinyVec<[u8; 16]>;

/// Describes an individual packet.
#[derive(Clone, PartialEq, Debug)]
pub enum Packet {
    /// Do nothing
    NoOp,

    /// Run emulator for `frames` frames
    #[allow(missing_docs)]
    RunFrames { frames: UnsignedInteger },

    /// Write RAM to the given address
    /// 
    /// How the address is interpreted is emulator-specific
    #[allow(missing_docs)]
    WriteMemory { address: UnsignedInteger, data: ByteVec },

    /// Set the current input.
    #[allow(missing_docs)]
    ChangeInput { data: InputBuffer },

    /// Set the current speed.
    #[allow(missing_docs)]
    ChangeSpeed { speed: Speed },

    /// Hard reset the console.
    ResetConsole,

    /// Load a save state at the given keyframe.
    #[allow(missing_docs)]
    RestoreState { keyframe_frame_index: UnsignedInteger },

    /// Describes a named point in the replay.
    #[allow(missing_docs)]
    Bookmark { metadata: BookmarkMetadata },

    /// Adds a keyframe so the replay can be scanned faster.
    #[allow(missing_docs)]
    Keyframe {
        metadata: KeyframeMetadata,
        state: ByteVec
    },

    /// Describes a compressed blob of memory.
    #[allow(missing_docs)]
    CompressedBlob {
        keyframes: Vec<KeyframeMetadata>,
        bookmarks: Vec<BookmarkMetadata>,
        compressed_data: ByteVec,
        uncompressed_size: UnsignedInteger
    }
}

/// Speed value that uses a fixed point number.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[repr(transparent)]
pub struct Speed {
    /// A fixed point number that, when divided by 256, will yield the speed value.
    pub speed_over_256: u16
}

impl Speed {
    /// Get the speed value from a multiplier.
    pub const fn from_multiplier_float(multiplier: f64) -> Self {
        Self {
            speed_over_256: (multiplier * 256.0) as u16
        }
    }
    /// Convert the speed value into a multiplier.
    pub const fn into_multiplier_float(self) -> f64 {
        match self.speed_over_256 {
            0 => 1.0 / 256.0,
            n => (n as f64) / 256.0
        }
    }
}

/// Payload for keyframes, not including save state data
#[derive(Clone, PartialEq, Debug, Default)]
pub struct KeyframeMetadata {
    /// Current input
    pub input: InputBuffer,

    /// Current speed
    pub speed: Speed,

    /// Number of elapsed frames
    pub elapsed_frames: UnsignedInteger,

    /// Number of "effective" emulator ticks passed multiplied by 256.
    /// 
    /// This may be scaled by the current speed.
    pub elapsed_emulator_ticks_over_256: UnsignedInteger
}

/// Payload for bookmarks
#[derive(Clone, PartialEq, Debug, Default)]
pub struct BookmarkMetadata {
    /// Name of the bookmark
    pub name: String,

    /// Number of elapsed frames
    pub elapsed_frames: UnsignedInteger
}

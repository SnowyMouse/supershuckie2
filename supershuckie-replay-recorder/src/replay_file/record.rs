//! Replay file recording functionality.
//!
//! See [`ReplayFileRecorder`] and [`NonBlockingReplayFileRecorder`].

use crate::replay_file::ReplayFileMetadata;
use crate::{BookmarkMetadata, ByteVec, InputBuffer, KeyframeMetadata, Packet, PacketIO, PacketWriteCommand, Speed, UnsignedInteger};
use alloc::string::String;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use alloc::format;
use zstd_sys::ZSTD_defaultCLevel;

#[cfg(not(feature = "std"))]
use spin::Lazy as LazyLock;

#[cfg(feature = "std")]
use std::sync::LazyLock;

#[cfg(feature = "std")]
mod thread;

#[cfg(feature = "std")]
pub use thread::*;

#[cfg(feature = "std")]
use std::{
    io::{Seek, SeekFrom, Write},
    fs::File
};

/// Records a replay file
///
/// IMPORTANT: To finish the stream, you must call [`ReplayFileRecorder::close`]. It is highly
/// recommended to also add a keyframe immediately before calling close so that the length of the
/// replay can be estimated accurately.
pub struct ReplayFileRecorder<Final: ReplayFileSink, Temp: ReplayFileSink> {
    settings: ReplayFileRecorderSettings,

    current_blob: Vec<u8>,
    current_blob_keyframes: Vec<KeyframeMetadata>,
    current_blob_bookmarks: Vec<BookmarkMetadata>,
    current_blob_offset: u64,

    frames_since_last_non_frames_packet: UnsignedInteger,
    elapsed_frames: UnsignedInteger,
    elapsed_ticks_over_256: UnsignedInteger,
    last_keyframe_frames: UnsignedInteger,

    all_keyframes: Vec<UnsignedInteger>,

    current_speed: Speed,
    current_input: InputBuffer,

    final_sink: Option<Final>,
    temporary_sink: Option<Temp>,

    poisoned: bool
}

/// Settings for [`ReplayFileRecorder`]
#[derive(Clone)]
pub struct ReplayFileRecorderSettings {
    /// Minimum uncompressed bytes per blob
    ///
    /// Default is [`DEFAULT_MINIMUM_UNCOMPRESSED_BYTES_PER_BLOB`]
    pub minimum_uncompressed_bytes_per_blob: usize,

    /// zstd compression level
    ///
    /// Default is [`DEFAULT_ZSTD_COMPRESSION_LEVEL`]
    pub compression_level: i32
}

/// Default minimum uncompressed bytes per blob
pub const DEFAULT_MINIMUM_UNCOMPRESSED_BYTES_PER_BLOB: usize = 512 * 1024 * 1024;

/// Default compression level
///
/// This is generally going to be equal to `3`.
pub static DEFAULT_ZSTD_COMPRESSION_LEVEL: LazyLock<i32> = LazyLock::new(|| unsafe { ZSTD_defaultCLevel() } as i32);

impl<Final: ReplayFileSink, Temp: ReplayFileSink> ReplayFileRecorder<Final, Temp> {
    /// Start a new replay file.
    pub fn new_with_metadata(
        replay_file_metadata: ReplayFileMetadata,
        patch_data: ByteVec,
        mut settings: ReplayFileRecorderSettings,
        starting_emulator_ticks_over_256: UnsignedInteger,
        starting_input: InputBuffer,
        starting_speed: Speed,
        initial_keyframe_state: ByteVec,
        mut final_sink: Final,
        mut temporary_sink: Temp
    ) -> Result<ReplayFileRecorder<Final, Temp>, ReplayFileWriteError> {
        if settings.minimum_uncompressed_bytes_per_blob == 0 {
            settings.minimum_uncompressed_bytes_per_blob = 1024 * 1024 * 512;
        }

        let mut metadata = replay_file_metadata
            .as_raw_header()
            .map_err(|e| ReplayFileWriteError::Other { explanation: Cow::Owned(e) })?;

        metadata.patch_data_length = u64::try_from(patch_data.len())
            .map_err(|_| ReplayFileWriteError::Other { explanation: Cow::Borrowed("patch data too large") })?;

        let metadata_bytes = metadata.as_bytes();
        let current_blob_offset = metadata_bytes.len() + patch_data.len();

        temporary_sink.write_bytes(metadata_bytes.as_slice())?;
        final_sink.write_bytes(metadata_bytes.as_slice())?;

        temporary_sink.write_bytes(patch_data.as_slice())?;
        final_sink.write_bytes(patch_data.as_slice())?;

        let mut recorder = ReplayFileRecorder {
            settings,
            elapsed_frames: 0,
            elapsed_ticks_over_256: 0,
            last_keyframe_frames: 0,
            current_speed: starting_speed,
            current_input: starting_input,
            current_blob: Vec::new(),
            current_blob_keyframes: Vec::new(),
            current_blob_bookmarks: Vec::new(),
            current_blob_offset: u64::try_from(current_blob_offset).expect("failed to read"),
            frames_since_last_non_frames_packet: 0,
            all_keyframes: Vec::new(),
            poisoned: false,
            final_sink: Some(final_sink),
            temporary_sink: Some(temporary_sink),
        };

        recorder.insert_keyframe(
            initial_keyframe_state,
            starting_emulator_ticks_over_256
        )?;

        Ok(recorder)
    }

    /// Returns `true` if the stream was closed.
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.final_sink.is_none()
    }

    /// Close the replay file recorder.
    ///
    /// You can no longer write to this.
    ///
    /// # Panics
    ///
    /// Panics if already closed.
    pub fn close(&mut self) -> Result<(Final, Temp), (Final, Temp, ReplayFileWriteError)> {
        if self.final_sink.is_none() || self.temporary_sink.is_none() {
            panic!("Already closed...")
        }

        let _ = self.next_blob();

        let (Some(final_sink), Some(temporary_sink)) = (self.final_sink.take(), self.temporary_sink.take()) else {
            unreachable!();
        };

        if let Err(e) = self.next_blob() {
            self.poisoned = true;
            return Err((final_sink, temporary_sink, e))
        }
        self.poisoned = true;
        Ok((final_sink, temporary_sink))
    }

    /// Returns true if an unrecoverable error occurred.
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Advance a new frame.
    pub fn next_frame(&mut self) {
        self.elapsed_frames += 1;
    }

    /// Add a bookmark.
    pub fn add_bookmark<S: Into<String>>(&mut self, name: S) -> Result<(), ReplayFileWriteError> {
        self.assert_not_closed()?;
        let bookmark_data = BookmarkMetadata {
            name: name.into(),
            elapsed_frames: self.elapsed_frames
        };

        self.current_blob_bookmarks.push(bookmark_data.clone());
        self.write_packet_data(&Packet::Bookmark {
            metadata: bookmark_data
        })
    }

    /// Add a new keyframe.
    ///
    /// Returns the frame index the keyframe is on.
    pub fn insert_keyframe(&mut self, state: ByteVec, elapsed_ticks_over_256: UnsignedInteger) -> Result<u64, ReplayFileWriteError> {
        assert!(self.elapsed_ticks_over_256 <= elapsed_ticks_over_256);
        self.assert_not_closed()?;

        self.elapsed_ticks_over_256 = elapsed_ticks_over_256;
        self.last_keyframe_frames = self.elapsed_frames;

        if self.current_blob.len() >= self.settings.minimum_uncompressed_bytes_per_blob {
            self.next_blob()?;
        }

        let metadata = KeyframeMetadata {
            input: self.current_input.clone(),
            speed: self.current_speed,
            elapsed_frames: self.elapsed_frames,
            elapsed_emulator_ticks_over_256: elapsed_ticks_over_256,
        };

        self.current_blob_keyframes.push(metadata.clone());

        self.write_packet_data(&Packet::Keyframe {
            metadata,
            state
        })?;

        Ok(self.elapsed_frames)
    }

    fn next_blob(&mut self) -> Result<(), ReplayFileWriteError> {
        self.do_with_poison(|this| {
            // Close off any pending frames
            this.write_run_frame_packet()?;

            let uncompressed_size = this.current_blob.len();
            let compressed = crate::compress_data(this.current_blob.as_slice(), this.settings.compression_level)
                .map_err(|e| ReplayFileWriteError::Other { explanation: Cow::Owned(format!("next_blob failed to compress: {e}")) })?;

            this.current_blob.clear();

            let keyframes_len = this.current_blob_keyframes.len();

            let compressed_blob = Packet::CompressedBlob {
                keyframes: core::mem::take(&mut this.current_blob_keyframes),
                bookmarks: core::mem::take(&mut this.current_blob_bookmarks),
                compressed_data: ByteVec::Heap(compressed),
                uncompressed_size: u64::try_from(uncompressed_size).expect("failed to convert uncompressed_size from usize to u64")
            };

            this.current_blob_keyframes.reserve(keyframes_len + 1024);

            let write_instructions = compressed_blob.write_packet_instructions();

            let current_blob_offset_old = this.current_blob_offset;

            let (final_sink, temporary_sink) = this.get_sinks();

            let written = final_sink.write_packet_data(&write_instructions)?;
            let written = u64::try_from(written).expect("failing to convert written packet data from usize to u64");
            temporary_sink.truncate(current_blob_offset_old)?;
            temporary_sink.write_packet_data(&write_instructions)?;

            this.current_blob_offset = current_blob_offset_old.checked_add(written).expect("overflowed adding current_blob_offset");

            Ok(())
        })
    }

    /// Set the current input.
    pub fn set_input(&mut self, input_buffer: InputBuffer) -> Result<(), ReplayFileWriteError> {
        if self.current_input == input_buffer {
            return Ok(())
        }

        self.write_packet_data(&Packet::ChangeInput { data: input_buffer })
    }

    /// Hard-reset the console.
    pub fn reset_console(&mut self) -> Result<(), ReplayFileWriteError> {
        self.write_packet_data(&Packet::ResetConsole)
    }

    /// Write RAM to an address.
    pub fn write_memory(&mut self, address: UnsignedInteger, data: ByteVec) -> Result<(), ReplayFileWriteError> {
        self.write_packet_data(&Packet::WriteMemory { address, data })
    }

    /// Set the current speed.
    pub fn set_speed(&mut self, speed: Speed) -> Result<(), ReplayFileWriteError> {
        if self.current_speed == speed {
            return Ok(())
        }

        self.write_packet_data(&Packet::ChangeSpeed { speed })
    }

    /// Load the keyframe at the given frame index.
    pub fn restore_state(&mut self, keyframe_frame_index: u64) -> Result<(), ReplayFileWriteError> {
        if self.all_keyframes.binary_search(&keyframe_frame_index).is_err() {
            return Err(ReplayFileWriteError::BadInput { explanation: Cow::Owned(format!("No keyframe at frame# {keyframe_frame_index}")) });
        }
        self.do_with_poison(|this| {
            this.write_packet_data(&Packet::RestoreState { keyframe_frame_index })
        })
    }

    fn write_packet_data<'a, P: PacketIO<'a>>(&mut self, what: &'a P) -> Result<(), ReplayFileWriteError> {
        self.do_with_poison(|this| {
            this.write_run_frame_packet()?;

            let instructions = what.write_packet_instructions();
            this.current_blob.write_packet_data(&instructions)?;
            this.temporary_sink.as_mut().expect("write_packet_data on None sink").write_packet_data(&instructions)?;
            Ok(())
        })
    }

    fn get_sinks(&mut self) -> (&mut Final, &mut Temp) {
        let final_sink = self.final_sink.as_mut().expect("can't get final sink (already closed?)");
        let temporary_sink = self.temporary_sink.as_mut().expect("can't get temp sink (already closed?)");

        (final_sink, temporary_sink)
    }

    fn write_run_frame_packet(&mut self) -> Result<(), ReplayFileWriteError> {
        let frames_since_last_non_frames = core::mem::take(&mut self.frames_since_last_non_frames_packet);
        if frames_since_last_non_frames > 0 {
            self.write_packet_data(&Packet::RunFrames { frames: frames_since_last_non_frames })?;
        }
        Ok(())
    }

    fn do_with_poison<T, F: FnOnce(&mut Self) -> Result<T, ReplayFileWriteError>>(&mut self, f: F) -> Result<T, ReplayFileWriteError> {
        self.assert_not_closed()?;
        if self.poisoned {
            return Err(ReplayFileWriteError::Poisoned)
        }
        self.poisoned = true;
        let result = f(self)?;
        self.poisoned = false;
        Ok(result)
    }

    fn assert_not_closed(&self) -> Result<(), ReplayFileWriteError> {
        if self.is_closed() {
            Err(ReplayFileWriteError::StreamClosed)
        }
        else {
            Ok(())
        }
    }
}

impl Default for ReplayFileRecorderSettings {
    fn default() -> Self {
        Self {
            minimum_uncompressed_bytes_per_blob: DEFAULT_MINIMUM_UNCOMPRESSED_BYTES_PER_BLOB,
            compression_level: *DEFAULT_ZSTD_COMPRESSION_LEVEL,
        }
    }
}

/// Describes something that can store bytes contiguously, making it suitable for a replay file.
pub trait ReplayFileSink {
    /// Writes bytes to the end of the sink.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ReplayFileWriteError>;

    /// Truncates the sink to the given size.
    fn truncate(&mut self, size: u64) -> Result<(), ReplayFileWriteError>;

    /// Writes the given packet data.
    fn write_packet_data(&mut self, instructions: &[PacketWriteCommand<'_>]) -> Result<usize, ReplayFileWriteError> {
        let mut written = 0usize;
        for i in instructions {
            let bytes = i.bytes();
            self.write_bytes(bytes)?;
            written += bytes.len();
        }
        Ok(written)
    }
}

impl ReplayFileSink for Vec<u8> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ReplayFileWriteError> {
        self.try_reserve(bytes.len()).map_err(|_| ReplayFileWriteError::Other { explanation: Cow::Borrowed("write_bytes failed to reserve memory") })?;
        self.extend_from_slice(bytes);
        Ok(())
    }

    #[inline]
    fn truncate(&mut self, size: u64) -> Result<(), ReplayFileWriteError> {
        self.truncate(usize::try_from(size).expect("converting u64 to usize should work when truncating"));
        Ok(())
    }

    fn write_packet_data(&mut self, instructions: &[PacketWriteCommand<'_>]) -> Result<usize, ReplayFileWriteError> {
        let mut total_len = 0usize;
        for i in instructions {
            total_len = total_len.saturating_add(i.bytes().len());
        }
        self.try_reserve(total_len).map_err(|_| ReplayFileWriteError::Other { explanation: Cow::Borrowed("write_packet_data failed to reserve memory") })?;
        for i in instructions {
            self.extend_from_slice(i.bytes())
        }
        Ok(total_len)
    }
}

#[cfg(feature = "std")]
impl ReplayFileSink for File {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ReplayFileWriteError> {
        self.write_all(bytes)?;
        Ok(())
    }

    fn truncate(&mut self, size: u64) -> Result<(), ReplayFileWriteError> {
        self.set_len(size)?;
        self.seek(SeekFrom::End(0))?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for ReplayFileWriteError {
    fn from(value: std::io::Error) -> Self {
        Self::Other { explanation: Cow::Owned(format!("I/O error: {value}")) }
    }
}

/// Describes an error that occurred when writing
#[derive(Clone, PartialEq, Debug)]
pub enum ReplayFileWriteError {
    /// Bad input was given. The stream might still be functional.
    #[allow(missing_docs)]
    BadInput { explanation: Cow<'static, str> },

    /// The stream has closed. The stream is no longer functional.
    StreamClosed,

    /// The stream was broken by a previous error. The stream is no longer functional.
    Poisoned,

    /// Some other error occurred. The stream is no longer functional.
    #[allow(missing_docs)]
    Other { explanation: Cow<'static, str> }
}

/// A null sink
///
/// Useful if you do not want a temporary buffer, for example
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct NullReplayFileSink;

impl ReplayFileSink for NullReplayFileSink {
    fn write_bytes(&mut self, _: &[u8]) -> Result<(), ReplayFileWriteError> {
        Ok(())
    }
    fn truncate(&mut self, _: u64) -> Result<(), ReplayFileWriteError> {
        Ok(())
    }
    fn write_packet_data(&mut self, instructions: &[PacketWriteCommand<'_>]) -> Result<usize, ReplayFileWriteError> {
        let mut len = 0usize;
        for i in instructions {
            len = len.saturating_add(i.bytes().len());
        }
        Ok(len)
    }
}

/// Object-safe wrapper for [`ReplayFileRecorder`]
///
/// See its documentation for what these functions do.
#[expect(missing_docs)]
pub trait ReplayFileRecorderFns: core::any::Any + 'static + Send {
    fn is_closed(&self) -> bool;
    fn close(&mut self) -> Result<(), ReplayFileWriteError>;
    fn next_frame(&mut self);
    fn add_bookmark(&mut self, name: String) -> Result<(), ReplayFileWriteError>;
    fn insert_keyframe(&mut self, state: ByteVec, elapsed_ticks_over_256: UnsignedInteger) -> Result<(), ReplayFileWriteError>;
    fn set_input(&mut self, input_buffer: InputBuffer) -> Result<(), ReplayFileWriteError>;
    fn reset_console(&mut self) -> Result<(), ReplayFileWriteError>;
    fn write_memory(&mut self, address: UnsignedInteger, data: ByteVec) -> Result<(), ReplayFileWriteError>;
    fn set_speed(&mut self, speed: Speed) -> Result<(), ReplayFileWriteError>;
    fn restore_state(&mut self, keyframe_frame_index: u64) -> Result<(), ReplayFileWriteError>;
}

impl<Final: ReplayFileSink + 'static + Send, Temp: ReplayFileSink + 'static + Send> ReplayFileRecorderFns for ReplayFileRecorder<Final, Temp> {
    #[inline]
    fn is_closed(&self) -> bool {
        self.is_closed()
    }

    #[inline]
    fn close(&mut self) -> Result<(), ReplayFileWriteError> {
        self.close().map_err(|e| e.2)?;
        Ok(())
    }

    #[inline]
    fn next_frame(&mut self) {
        self.next_frame()
    }

    #[inline]
    fn add_bookmark(&mut self, name: String) -> Result<(), ReplayFileWriteError> {
        self.add_bookmark(name)
    }

    #[inline]
    fn insert_keyframe(&mut self, state: ByteVec, elapsed_ticks_over_256: UnsignedInteger) -> Result<(), ReplayFileWriteError> {
        self.insert_keyframe(state, elapsed_ticks_over_256)?;
        Ok(())
    }

    #[inline]
    fn set_input(&mut self, input_buffer: InputBuffer) -> Result<(), ReplayFileWriteError> {
        self.set_input(input_buffer)
    }

    #[inline]
    fn reset_console(&mut self) -> Result<(), ReplayFileWriteError> {
        self.reset_console()
    }

    #[inline]
    fn write_memory(&mut self, address: UnsignedInteger, data: ByteVec) -> Result<(), ReplayFileWriteError> {
        self.write_memory(address, data)
    }

    #[inline]
    fn set_speed(&mut self, speed: Speed) -> Result<(), ReplayFileWriteError> {
        self.set_speed(speed)
    }

    #[inline]
    fn restore_state(&mut self, keyframe_frame_index: u64) -> Result<(), ReplayFileWriteError> {
        self.restore_state(keyframe_frame_index)
    }
}

fn _ensure_replay_file_recorder_fns_is_dyn_compatible(_fns: &dyn ReplayFileRecorderFns) {}

// TODO: WRITE UNIT TESTS

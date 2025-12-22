use crate::emulator::{EmulatorCore, Input, PartialReplayRecordMetadata, ScreenData};
use crate::{ReplayPlayerAttachError, Speed};
use crate::{SuperShuckieCore, SuperShuckieRapidFire};
use std::borrow::ToOwned;
use std::boxed::Box;
use std::fs::File;
use std::string::String;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, TryLockError, Weak};
use std::time::Duration;
use std::vec::Vec;
use std::format;
#[cfg(feature = "pokeabyte")]
use supershuckie_pokeabyte_integration::PokeAByteIntegrationServer;
use supershuckie_replay_recorder::replay_file::playback::ReplayFilePlayer;
use supershuckie_replay_recorder::UnsignedInteger;

/// A (mostly) non-blocking, threaded wrapper for [`SuperShuckieCore`].
pub struct ThreadedSuperShuckieCore {
    screens: Arc<Mutex<Vec<ScreenData>>>,
    sender: Sender<ThreadCommand>,
    receiver_close: Receiver<()>,

    frame_count: Arc<AtomicU32>,
    elapsed_milliseconds: Arc<AtomicU32>,

    playback_total_frames: UnsignedInteger,
    playback_total_milliseconds: UnsignedInteger,
}

impl ThreadedSuperShuckieCore {
    /// Wrap the given `core`.
    pub fn new(emulator_core: Box<dyn EmulatorCore>) -> Self {
        let frame_count = Arc::new(AtomicU32::new(0));
        let screens = Arc::new(Mutex::new(emulator_core.get_screens().to_vec()));
        let (sender, receiver) = channel();
        let (sender_close, receiver_close) = channel();

        let replay_milliseconds = Arc::new(AtomicU32::new(0));
        let playback_total_frames = 0;
        let playback_total_milliseconds = 0;

        {
            let frame_count = frame_count.clone();
            let screens = Arc::downgrade(&screens);
            let replay_milliseconds = replay_milliseconds.clone();
            let _ = std::thread::Builder::new().name("ThreadedSuperShuckieCore".to_owned()).spawn(move || {
                ThreadedSuperShuckieCoreThread {
                    screens,
                    screens_queued: emulator_core.get_screens().to_vec(),
                    screen_ready_for_copy: false,
                    is_running: false,
                    core: SuperShuckieCore::new(emulator_core),
                    pokeabyte_integration: None,
                    receiver,
                    sender_close,
                    frame_count,
                    replay_milliseconds
                }.run_thread();
            });
        }

        Self {
            sender,
            screens,
            receiver_close,
            frame_count,
            elapsed_milliseconds: replay_milliseconds,
            playback_total_frames,
            playback_total_milliseconds
        }
    }

    /// Get the elapsed frame count.
    ///
    /// This can be called to ensure that a unique frame is ready to be read. Note, however, that
    /// this number may be slightly outdated.
    pub fn get_elapsed_frames(&self) -> u32 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Read the screens.
    ///
    /// Note that while this function is running, the screen buffer will be blocked from being
    /// updated and may not be immediately updated until later.
    pub fn read_screens<T, F: FnOnce(&[ScreenData]) -> T>(&self, reader: F) -> T {
        let lock = self.screens.lock().expect("screen mutex is poisoned");
        reader(lock.as_slice())
    }

    /// Start running continuously.
    pub fn start(&self) {
        self.sender.send(ThreadCommand::Start)
            .expect("Start - the core thread has crashed");
    }

    /// Pause running.
    pub fn pause(&self) {
        self.sender.send(ThreadCommand::Pause)
            .expect("Pause - the core thread has crashed");
    }

    /// Attach/detach a Poke-A-Byte integration server.
    pub fn set_pokeabyte_enabled(&self, enabled: bool) -> Result<(), String> {
        let (sender, receiver) = channel();

        self.sender.send(ThreadCommand::SetPokeAByteEnabled(enabled, sender))
            .expect("SetPokeAByteEnabled - the core thread has crashed");

        receiver.recv().ok().unwrap_or(Ok(()))
    }

    /// Stop recording replay.
    pub fn start_recording_replay(&self, metadata: PartialReplayRecordMetadata<File, File>) {
        self.sender.send(ThreadCommand::StartRecordingReplay(metadata))
            .expect("StopRecordingReplay - the core thread has crashed");
    }

    /// Stop recording replay.
    pub fn stop_recording_replay(&self) -> bool {
        let (sender, receiver) = channel();

        self.sender.send(ThreadCommand::StopRecordingReplay(sender))
            .expect("StopRecordingReplay - the core thread has crashed");

        receiver.recv().ok().unwrap_or(false)
    }

    /// Enqueue an input.
    pub fn enqueue_input(&self, input: Input) {
        self.sender.send(ThreadCommand::EnqueueInput(input))
            .expect("EnqueueInput - the core thread has crashed");
    }

    /// Set the speed.
    pub fn set_speed(&self, speed: Speed) {
        self.sender.send(ThreadCommand::SetSpeed(speed))
            .expect("SetSpeed - the core thread has crashed");
    }

    /// Set the speed.
    pub fn hard_reset(&self) {
        self.sender.send(ThreadCommand::HardReset)
            .expect("HardReset - the core thread has crashed");
    }

    /// Set the rapid fire input.
    pub fn set_rapid_fire_input(&self, input: Option<SuperShuckieRapidFire>) {
        self.sender.send(ThreadCommand::SetRapidFireInput(input))
            .expect("SetRapidFireInput - the core thread has crashed");
    }

    /// Set the toggle input.
    pub fn set_toggled_input(&self, input: Option<Input>) {
        self.sender.send(ThreadCommand::SetToggledInput(input))
            .expect("SetToggledInput - the core thread has crashed");
    }

    /// Create a save state.
    ///
    /// Returns `None` if no save state could be created for some unknown reason.
    ///
    /// NOTE: This is blocking.
    pub fn create_save_state(&self) -> Option<Vec<u8>> {
        let (sender, receiver) = channel();
        self.sender.send(ThreadCommand::CreateSaveState(sender))
            .expect("CreateSaveState - the core thread has crashed");
        receiver.recv().ok()
    }

    /// Load a save state.
    pub fn load_save_state(&self, state: Vec<u8>) {
        self.sender.send(ThreadCommand::LoadSaveState(state))
            .expect("LoadSaveState - the core thread has crashed");
    }

    /// Get SRAM.
    ///
    /// Returns `None` if SRAM could not be read for some unknown reason.
    ///
    /// NOTE: This is blocking.
    pub fn get_sram(&self) -> Option<Vec<u8>> {
        let (sender, receiver) = channel();
        self.sender.send(ThreadCommand::SaveSRAM(sender))
            .expect("SaveSRAM - the core thread has crashed");
        receiver.recv().ok()
    }

    /// Get the number of milliseconds a replay has been recorded.
    #[inline]
    pub fn get_elapsed_milliseconds(&self) -> u32 {
        self.elapsed_milliseconds.load(Ordering::Relaxed)
    }

    /// Get the total number of frames in the current playback.
    #[inline]
    pub fn get_playback_total_frames(&self) -> u32 {
        self.playback_total_frames as u32
    }

    /// Get the total number of frames in the current playback.
    #[inline]
    pub fn get_playback_total_milliseconds(&self) -> u32 {
        self.playback_total_milliseconds as u32
    }

    /// Load the replay.
    pub fn attach_replay_player(&mut self, mut player: ReplayFilePlayer, allow_mismatch: bool) -> Result<(), ReplayPlayerAttachError> {
        player.enable_threading();

        let total_ticks = player.get_total_ticks_over_256();
        let total_frames = player.get_total_frames();

        let (sender, receiver) = channel();

        self.sender.send(ThreadCommand::AttachReplayPlayer {
            player,
            allow_mismatched: allow_mismatch,
            errors: sender
        }).expect("AttachReplayPlayer - the core thread has crashed");

        match receiver.recv() {
            Err(_) => {
                self.playback_total_frames = total_frames;
                self.playback_total_milliseconds = total_ticks;
                Ok(())
            },
            Ok(n) => Err(n)
        }
    }

    /// Detach a replay
    pub fn detach_replay_player(&mut self) {
        self.playback_total_frames = 0;
        self.playback_total_milliseconds = 0;
        self.sender.send(ThreadCommand::DetachReplayPlayer)
            .expect("DetachReplayPlayer - the core thread has crashed")
    }
}

impl Drop for ThreadedSuperShuckieCore {
    fn drop(&mut self) {
        // we couldn't really care less if these succeed or fail; we just want to ensure that
        // the replay file is closed, and it should be (if it didn't error)
        let _ = self.sender.send(ThreadCommand::Close);
        let _ = self.receiver_close.recv();
    }
}

// TODO: Option to run just a single frame? Maybe also skip around a replay file to a given
//       keyframe...
enum ThreadCommand {
    Start,
    Pause,
    SetPokeAByteEnabled(bool, Sender<Result<(), String>>),
    StartRecordingReplay(PartialReplayRecordMetadata<File, File>),
    StopRecordingReplay(Sender<bool>),
    AttachReplayPlayer {
        player: ReplayFilePlayer,
        allow_mismatched: bool,
        errors: Sender<ReplayPlayerAttachError>
    },
    DetachReplayPlayer,
    EnqueueInput(Input),
    SetRapidFireInput(Option<SuperShuckieRapidFire>),
    SetToggledInput(Option<Input>),
    SetSpeed(Speed),
    HardReset,
    CreateSaveState(Sender<Vec<u8>>),
    LoadSaveState(Vec<u8>),
    SaveSRAM(Sender<Vec<u8>>),
    Close
}

struct ThreadedSuperShuckieCoreThread {
    screens: Weak<Mutex<Vec<ScreenData>>>,

    screens_queued: Vec<ScreenData>,
    screen_ready_for_copy: bool,
    frame_count: Arc<AtomicU32>,
    replay_milliseconds: Arc<AtomicU32>,

    core: SuperShuckieCore,
    receiver: Receiver<ThreadCommand>,
    is_running: bool,
    pokeabyte_integration: Option<PokeAByteIntegrationServer>,
    sender_close: Sender<()>
}

impl ThreadedSuperShuckieCoreThread {
    fn run_thread(mut self) {
        loop {
            if let Ok(cmd) = self.receiver.try_recv() {
                if matches!(cmd, ThreadCommand::Close) {
                    break
                }

                self.handle_command(cmd);
                continue
            }

            self.refresh_screen_data(false);
            self.update_queued_screens();
            self.handle_pokeabyte_integration();
            self.replay_milliseconds.store(self.core.get_recording_milliseconds(), Ordering::Relaxed);

            if self.is_running {
                self.core.run();
            }
            else {
                // unfortunately we can't just block until we're running again because we still need
                // to handle pokeabyte writes
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        self.core.stop_recording_replay();
        self.pokeabyte_integration = None;

        let _ = self.sender_close.send(());
    }

    /// If the mutex was blocked, we can copy it in when it's no longer blocked.
    fn update_queued_screens(&mut self) {
        if !self.screen_ready_for_copy {
            return
        }

        let Some(screen_data) = self.screens.upgrade() else {
            panic!("update_queued_screens Can't get screen_data: owning thread must have crashed");
        };

        let mut out_screens = match screen_data.try_lock() {
            Ok(n) => n,
            Err(TryLockError::WouldBlock) => return,
            Err(e) => panic!("update_queued_screens Can't get screens mutex: {e}")
        };

        self.screen_ready_for_copy = false;

        let in_screens = &mut self.screens_queued;
        core::mem::swap(in_screens, &mut *out_screens);

        self.frame_count.store(self.core.total_frames as u32, Ordering::Relaxed);
    }

    /// Attempt to copy the screen data, or store it for later.
    fn refresh_screen_data(&mut self, force: bool) {
        if !force && self.is_running && self.core.mid_frame {
            return
        }

        let Some(screen_data) = self.screens.upgrade() else {
            panic!("refresh_screen_data Can't get screen_data: owning thread must have crashed");
        };

        let mut out_screens_maybe = screen_data.try_lock();

        let out_screens_result = match out_screens_maybe.as_mut() {
            Ok(n) => {
                self.screen_ready_for_copy = false;

                // this is safe to update early since we have the mutex locked
                self.frame_count.store(self.core.total_frames as u32, Ordering::Relaxed);
                &mut *n
            },
            Err(TryLockError::WouldBlock) => {
                self.screen_ready_for_copy = true;
                &mut self.screens_queued
            },
            Err(e) => panic!("refresh_screen_data Can't get screens mutex: {e}")
        };

        self.core.core.swap_screen_data(out_screens_result.as_mut_slice());
    }

    /// Update RAM read/writes
    fn handle_pokeabyte_integration(&mut self) {
        let Some(integration) = self.pokeabyte_integration.as_ref() else {
            return
        };

        let mut session_lock = integration.get_session();
        let Some(session) = session_lock.as_mut() else {
            return;
        };

        for write in &mut session.writes {
            self.core.enqueue_write(write.address as u32, write.data);
        }

        // don't update reads mid-frame; it's too slow
        if self.core.mid_frame && self.is_running {
            return;
        }

        // handle frame skipping unless we're paused
        if self.is_running && let Some(skipping) = session.config.frame_skip && self.core.total_frames % ((skipping as u64) + 1) != 0 {
            return
        }

        // SAFETY: "Only one way to find out"
        let ram = unsafe { session.shared_memory.get_memory_mut() };
        for read in &session.config.blocks {
            let into = ram.get_mut(read.range.clone()).expect("read range was wrong (this should have been checked!)");
            let _ = self.core.get_core().read_ram(read.game_address, into); // TODO: handle this?
        }
    }

    fn handle_command(&mut self, command: ThreadCommand) {
        match command {
            ThreadCommand::Start => {
                self.is_running = true;
            }
            ThreadCommand::Pause => {
                self.is_running = false;
            }
            ThreadCommand::SetPokeAByteEnabled(enabled, err) => {
                if !enabled && self.pokeabyte_integration.is_some() {
                    self.pokeabyte_integration = None;
                    let _ = err.send(Ok(()));
                }
                else if enabled {
                    let integration = match PokeAByteIntegrationServer::begin_listen() {
                        Ok(n) => {
                            let _ = err.send(Ok(()));
                            n
                        },
                        Err(e) => {
                            let _ = err.send(Err(format!("{e:?}")));
                            return
                        }
                    };
                    self.pokeabyte_integration = Some(integration)
                } else {
                    let _ = err.send(Ok(()));
                }
            }
            ThreadCommand::StartRecordingReplay(metadata) => {
                // FIXME: error if this fails
                self.core.start_recording_replay(metadata).expect("FAILED TO START RECORDING REPLAY OH NO");
            }
            ThreadCommand::StopRecordingReplay(sender) => {
                let _ = sender.send(self.core.stop_recording_replay() == Some(true));
            }
            ThreadCommand::EnqueueInput(input) => {
                self.core.enqueue_input(input);
            }
            ThreadCommand::SetSpeed(speed) => {
                self.core.set_speed(speed);
            }
            ThreadCommand::SetRapidFireInput(input) => {
                self.core.set_rapid_fire_input(input);
            }
            ThreadCommand::SetToggledInput(input) => {
                self.core.set_toggled_input(input);
            }
            ThreadCommand::HardReset => {
                self.core.hard_reset();
            }
            ThreadCommand::CreateSaveState(sender) => {
                self.core.finish_current_frame();
                let _ = sender.send(self.core.create_save_state());
            }
            ThreadCommand::LoadSaveState(state) => {
                self.core.load_save_state(&state);
            }
            ThreadCommand::SaveSRAM(sender) => {
                let _ = sender.send(self.core.save_sram());
            }
            ThreadCommand::Close => {
                unreachable!("handle_command(ThreadCommand::Close) should not happen")
            },
            ThreadCommand::AttachReplayPlayer { player, allow_mismatched, errors } => {
                if let Err(e) = self.core.attach_replay_player(player, allow_mismatched) {
                    let _ = errors.send(e);
                }
            }
            ThreadCommand::DetachReplayPlayer => {
                self.core.detach_replay_player();
            }
        }
    }
}

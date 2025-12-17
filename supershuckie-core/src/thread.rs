use crate::emulator::{EmulatorCore, Input, ScreenData};
use crate::{SuperShuckieCore, SuperShuckieRapidFire};
use std::boxed::Box;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, TryLockError, Weak};
use std::time::Duration;
use std::vec::Vec;
use std::borrow::ToOwned;
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "pokeabyte")]
use supershuckie_pokeabyte_integration::PokeAByteIntegrationServer;
use supershuckie_replay_recorder::replay_file::record::ReplayFileRecorderFns;
use crate::Speed;

/// A (mostly) non-blocking, threaded wrapper for [`SuperShuckieCore`].
pub struct ThreadedSuperShuckieCore {
    screens: Arc<Mutex<Vec<ScreenData>>>,
    sender: Sender<ThreadCommand>,
    frame_count: Arc<AtomicU32>,
    receiver_close: Receiver<()>
}

impl ThreadedSuperShuckieCore {
    /// Wrap the given `core`.
    pub fn new(emulator_core: Box<dyn EmulatorCore>) -> Self {
        let frame_count = Arc::new(AtomicU32::new(0));
        let screens = Arc::new(Mutex::new(emulator_core.get_screens().to_vec()));
        let (sender, receiver) = std::sync::mpsc::channel();
        let (sender_close, receiver_close) = std::sync::mpsc::channel();

        {
            let frame_count = frame_count.clone();
            let screens = Arc::downgrade(&screens);
            let _ = std::thread::Builder::new().name("ThreadedSuperShuckieCore".to_owned()).spawn(move || {
                ThreadedSuperShuckieCoreThread {
                    screens,
                    screens_queued: emulator_core.get_screens().to_vec(),
                    screen_ready_for_copy: false,
                    is_running: false,
                    core: SuperShuckieCore::new(emulator_core),
                    integration: None,
                    receiver,
                    sender_close,
                    frame_count
                }.run_thread();
            });
        }

        Self {
            sender,
            screens,
            receiver_close,
            frame_count
        }
    }

    /// Get the elapsed frame count.
    ///
    /// This can be called to ensure that a unique frame is ready to be read. Note, however, that
    /// this number may be slightly outdated.
    pub fn get_frame_count(&self) -> u32 {
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
        let _ = self.sender.send(ThreadCommand::Start)
            .expect("Start - the core thread has crashed");
    }

    /// Pause running.
    pub fn pause(&self) {
        let _ = self.sender.send(ThreadCommand::Pause)
            .expect("Pause - the core thread has crashed");
    }

    /// Attach a Poke-A-Byte integration server.
    pub fn attach_pokeabyte_server(&self, integration: Option<PokeAByteIntegrationServer>) {
        let _ = self.sender.send(ThreadCommand::AttachPokeAByteIntegration(integration))
            .expect("AttachPokeAByteIntegration - the core thread has crashed");
    }

    /// Attach a replay file recorder.
    pub fn attach_file_recorder(&self, recorder: Option<Box<dyn ReplayFileRecorderFns>>) {
        let _ = self.sender.send(ThreadCommand::AttachReplayFileRecorder(recorder))
            .expect("AttachPokeAByteIntegration - the core thread has crashed");
    }

    /// Enqueue an input.
    pub fn enqueue_input(&self, input: Input) {
        let _ = self.sender.send(ThreadCommand::EnqueueInput(input))
            .expect("EnqueueInput - the core thread has crashed");
    }

    /// Set the speed.
    pub fn set_speed(&self, speed: Speed) {
        let _ = self.sender.send(ThreadCommand::SetSpeed(speed));
    }

    /// Set the speed.
    pub fn hard_reset(&self) {
        let _ = self.sender.send(ThreadCommand::HardReset);
    }

    /// Set the rapid fire input.
    pub fn set_rapid_fire_input(&self, input: Option<SuperShuckieRapidFire>) {
        let _ = self.sender.send(ThreadCommand::SetRapidFireInput(input));
    }

    /// Set the toggle input.
    pub fn set_toggled_input(&self, input: Option<Input>) {
        let _ = self.sender.send(ThreadCommand::SetToggledInput(input));
    }

    /// Create a save state.
    ///
    /// Returns `None` if no save state could be created for some unknown reason.
    ///
    /// NOTE: This is blocking.
    pub fn create_save_state(&self) -> Option<Vec<u8>> {
        let (sender, receiver) = channel();
        let _ = self.sender.send(ThreadCommand::CreateSaveState(sender));
        receiver.recv().ok()
    }

    /// Load a save state.
    pub fn load_save_state(&self, state: Vec<u8>) {
        let _ = self.sender.send(ThreadCommand::LoadSaveState(state));
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
    AttachPokeAByteIntegration(Option<PokeAByteIntegrationServer>),
    AttachReplayFileRecorder(Option<Box<dyn ReplayFileRecorderFns>>),
    EnqueueInput(Input),
    SetRapidFireInput(Option<SuperShuckieRapidFire>),
    SetToggledInput(Option<Input>),
    SetSpeed(Speed),
    HardReset,
    CreateSaveState(Sender<Vec<u8>>),
    LoadSaveState(Vec<u8>),
    Close
}

struct ThreadedSuperShuckieCoreThread {
    screens: Weak<Mutex<Vec<ScreenData>>>,

    screens_queued: Vec<ScreenData>,
    screen_ready_for_copy: bool,
    frame_count: Arc<AtomicU32>,

    core: SuperShuckieCore,
    receiver: Receiver<ThreadCommand>,
    is_running: bool,
    integration: Option<PokeAByteIntegrationServer>,
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

            if self.is_running {
                self.core.run();
            }
            else {
                // unfortunately we can't just block until we're running again because we still need
                // to handle pokeabyte writes
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        self.core.attach_replay_file_recorder(None);
        self.integration = None;

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

        let in_screens = &self.screens_queued;
        for (in_screen, out_screen) in in_screens.iter().zip(out_screens.iter_mut()) {
            out_screen.pixels.copy_from_slice(in_screen.pixels.as_slice());
        }

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

        let in_screens = self.core.core.get_screens();
        debug_assert_eq!(out_screens_result.len(), in_screens.len(), "Screen count has changed!");

        for (in_screen, out_screen) in in_screens.iter().zip(out_screens_result.iter_mut()) {
            debug_assert_eq!(in_screen.width, out_screen.width, "Screen width has changed!");
            debug_assert_eq!(in_screen.height, out_screen.height, "Screen height has changed!");
            debug_assert_eq!(in_screen.pixels.len(), out_screen.pixels.len(), "Screen pixel count has changed!");
            debug_assert_eq!(in_screen.encoding, out_screen.encoding, "Screen pixel encoding has changed!");
            out_screen.pixels.copy_from_slice(in_screen.pixels.as_slice());
        }
    }

    /// Update RAM read/writes
    fn handle_pokeabyte_integration(&mut self) {
        let Some(integration) = self.integration.as_ref() else {
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
            ThreadCommand::AttachPokeAByteIntegration(integration) => {
                self.integration = integration;
            }
            ThreadCommand::AttachReplayFileRecorder(recorder) => {
                self.core.attach_replay_file_recorder(recorder);
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
                let _ = sender.send(self.core.core.create_save_state());
            }
            ThreadCommand::LoadSaveState(state) => {
                self.core.load_save_state(&state);
            }
            ThreadCommand::Close => {
                unreachable!("handle_command(ThreadCommand::Close) should not happen")
            }
        }
    }
}

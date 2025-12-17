pub mod util;
pub mod settings;

use crate::settings::*;
use crate::util::UTF8CString;
use std::ffi::CStr;
use std::num::{NonZeroU64, NonZeroU8};
use std::path::{Path, PathBuf};
use supershuckie_core::emulator::{EmulatorCore, GameBoyColor, Input, Model, NullEmulatorCore, ScreenData};
use supershuckie_core::{Speed, SuperShuckieRapidFire, ThreadedSuperShuckieCore};

const SETTINGS_FILE: &str = "settings.json";
const SAVE_STATE_EXTENSION: &str = "save_state";

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SuperShuckieEmulatorType {
    GameBoy,
    GameBoyColor
}

pub struct SuperShuckieFrontend {
    core: ThreadedSuperShuckieCore,
    core_metadata: CoreMetadata,

    callbacks: Box<dyn SuperShuckieFrontendCallbacks>,

    user_dir: PathBuf,
    frame_count: u32,

    loaded_rom_data: Option<Vec<u8>>,

    current_input: Input,
    current_rapid_fire_input: Option<SuperShuckieRapidFire>,
    current_toggled_input: Option<Input>,
    current_save_state_history: Vec<Vec<u8>>,
    current_save_state_history_position: usize,

    rom_name: Option<UTF8CString>,
    save_file: Option<UTF8CString>,

    settings: Settings
}

impl SuperShuckieFrontend {
    pub fn new<P: AsRef<Path>>(user_dir: P, callbacks: Box<dyn SuperShuckieFrontendCallbacks>) -> Self {
        let user_dir = user_dir.as_ref().to_owned();

        // FIXME: Check this
        let settings = try_to_init_user_dir_and_get_settings(user_dir.as_ref()).expect("failed to init user_dir");

        let mut s = Self {
            core: ThreadedSuperShuckieCore::new(Box::new(NullEmulatorCore)),
            core_metadata: CoreMetadata { emulator_type: None },
            user_dir,
            rom_name: None,
            save_file: None,
            loaded_rom_data: None,
            frame_count: 0,
            current_rapid_fire_input: None,
            current_toggled_input: None,
            callbacks,
            settings,
            current_input: Input::default(),
            current_save_state_history: Vec::new(),
            current_save_state_history_position: 0
        };

        s.unload_rom();

        s
    }

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }

    /// Create a save state.
    ///
    /// If `name` is set, that name will be used.
    ///
    /// Returns the name of the save state if created.
    pub fn create_save_state(&mut self, name: Option<&str>) -> Result<UTF8CString, UTF8CString> {
        if !self.is_game_running() {
            return Err("Game not running".into())
        }

        let current_rom_name = self.get_current_rom_name().expect("no rom name when game is running in create_save_state");
        let save_states_dir = self.get_save_states_dir_for_rom(current_rom_name);

        let path = match name {
            Some(name) => save_states_dir.join(format!("{name}.{SAVE_STATE_EXTENSION}")),
            None => {
                let current_save_name = self.get_current_save_name().expect("no save name when game is running in create_save_state");
                let mut i = 0u64;
                loop {
                    let path = save_states_dir.join(format!("{current_save_name}-{i}.{SAVE_STATE_EXTENSION}"));
                    if !path.exists() {
                        break path
                    }
                    i = i.checked_add(1).ok_or_else(|| UTF8CString::from_str("Maximum number of generic save states reached."))?;
                }
            }
        };

        let state = self.create_save_state_now();

        std::fs::write(&path, state)
            .map_err(|e| format!("Failed to write the save state to disk: {e}").into())
            .map(|_| path.file_stem().expect("save state name should exist").to_str().expect("save state should be utf-8").into())
    }

    /// Loads a save state with the given name if it exists.
    ///
    /// If it does, and it is successfully loaded, `Ok(true)` is returned.
    ///
    /// If it does not exist, `Ok(false)` is returned.
    pub fn load_save_state_if_exists(&mut self, name: &str) -> Result<bool, UTF8CString> {
        if !self.is_game_running() {
            return Err("Game not running".into())
        }

        let current_rom_name = self.get_current_rom_name().expect("no rom name when game is running in load_save_state");
        let save_states_dir = self.get_save_states_dir_for_rom(current_rom_name);
        let save_state_file = save_states_dir.join(format!("{name}.{SAVE_STATE_EXTENSION}"));

        if !save_state_file.is_file() {
            return Ok(false)
        }

        self.push_save_state_history();

        let save_state = std::fs::read(save_state_file).map_err(|e| format!("Failed to load save state {name}: {e}"))?;
        self.core.load_save_state(save_state);
        Ok(true)
    }

    fn push_save_state_history(&mut self) {
        self.current_save_state_history.truncate(self.current_save_state_history_position);
        self.current_save_state_history.push(self.create_save_state_now());

        while self.current_save_state_history.len() > self.settings.emulation.max_save_state_history.get() {
            self.current_save_state_history.remove(0);
        }

        self.current_save_state_history_position = self.current_save_state_history.len();

    }

    fn create_save_state_now(&self) -> Vec<u8> {
        self.core.create_save_state().expect("Failed to create a save state for an unknown reason (this is a bug!).") // TODO: handle this failing?
    }

    /// Undo loading a save state, loading the state before loading the save state.
    pub fn undo_load_save_state(&mut self) -> bool {
        if self.current_save_state_history_position == 0 {
            return false // no more to go
        }

        let backup = self.create_save_state_now();
        self.current_save_state_history_position -= 1;

        let history = &mut self.current_save_state_history[self.current_save_state_history_position];
        let state_to_load = std::mem::replace(history, backup);

        self.core.load_save_state(state_to_load);
        true
    }

    /// Redo loading a save state, loading the save state before undoing loading the save state.
    pub fn redo_load_save_state(&mut self) -> bool {
        if self.current_save_state_history_position == self.current_save_state_history.len() {
            return false // no more to go
        }

        let backup = self.create_save_state_now();

        let history = &mut self.current_save_state_history[self.current_save_state_history_position];
        self.current_save_state_history_position += 1;

        let state_to_load = std::mem::replace(history, backup);

        self.core.load_save_state(state_to_load);
        true
    }

    pub fn set_button_input(&mut self, control: &ControlSetting, pressed: bool) {
        if control.control.is_button() {
            match control.modifier {
                ControlModifier::Normal => {
                    control.control.set_for_input(&mut self.current_input, pressed);
                    self.core.enqueue_input(self.current_input);
                },
                ControlModifier::Rapid => {
                    if self.current_rapid_fire_input.is_none() {
                        if !pressed {
                            return
                        }

                        let mut new_rapid_fire = SuperShuckieRapidFire::default();
                        new_rapid_fire.hold_length = unsafe { NonZeroU64::new_unchecked(3) };
                        new_rapid_fire.interval = unsafe { NonZeroU64::new_unchecked(3) };
                        self.current_rapid_fire_input = Some(new_rapid_fire);
                    }

                    let Some(input) = self.current_rapid_fire_input.as_mut() else { unreachable!("we just enabled rapid fire input...!") };
                    control.control.set_for_input(&mut input.input, pressed);
                    if !pressed && input.input.is_empty() {
                        self.current_rapid_fire_input = None;
                    }
                    self.core.set_rapid_fire_input(self.current_rapid_fire_input);
                },
                ControlModifier::Toggle => {
                    if !pressed {
                        return
                    }
                    
                    if self.current_toggled_input.is_none() {
                        self.current_toggled_input = Some(Input::new());
                    }

                    let Some(input) = self.current_toggled_input.as_mut() else { unreachable!("we just enabled toggled input...!") };
                    control.control.invert_for_input(input);
                    if !pressed && input.is_empty() {
                        self.current_toggled_input = None;
                    }
                    self.core.set_toggled_input(self.current_toggled_input);
                }
            }
        }
        else if self.is_game_running() {
            match control.control {
                Control::Turbo => self.apply_turbo(pressed.then_some(1.0).unwrap_or(0.0)),
                Control::Reset => if pressed {
                    self.core.hard_reset();
                }
                Control::Pause => if pressed && self.is_game_running() {
                    self.set_paused(!self.settings.emulation.paused);
                }

                Control::A => unreachable!(),
                Control::B => unreachable!(),
                Control::Start => unreachable!(),
                Control::Select => unreachable!(),
                Control::Up => unreachable!(),
                Control::Down => unreachable!(),
                Control::Left => unreachable!(),
                Control::Right => unreachable!(),
                Control::L => unreachable!(),
                Control::R => unreachable!(),
                Control::X => unreachable!(),
                Control::Y => unreachable!(),
            }
        }
    }

    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) -> Result<(), UTF8CString> {
        let path = path.as_ref();

        let Some(filename) = path.file_name().and_then(|i| i.to_str()) else {
            return Err(format!(
                "{} does not appear to be a valid ROM file (missing filename)",
                path.display()
            ).into())
        };

        let Some(extension) = path.extension().and_then(|i| i.to_str()) else {
            return Err(format!("{filename} does not appear to be a valid ROM file (missing extension)").into())
        };

        let data = std::fs::read(path).map_err(|e| {
            format!("Failed to read ROM at {filename}: {e}")
        })?;

        let emulator_to_use = match extension.to_lowercase().as_str() {
            "gb" | "gbc" => {
                match self.settings.game_boy_settings.gbc_mode {
                    GameBoyMode::AlwaysGBC => SuperShuckieEmulatorType::GameBoyColor,
                    GameBoyMode::AlwaysGB => SuperShuckieEmulatorType::GameBoy,

                    // TODO: check header (the extension will not help)
                    GameBoyMode::GBInGBMode => todo!(),
                }
            },
            unknown => return Err(format!("Unknown or unsupported ROM file type .{unknown}").into())
        };

        self.create_userdata_for_rom(filename)?;
        self.unload_rom();
        self.loaded_rom_data = Some(data);
        self.rom_name = Some(UTF8CString::from_str(filename));
        self.core_metadata.emulator_type = Some(emulator_to_use);
        self.save_file = Some(self.get_current_save_file_name_for_rom(filename));
        self.reload_rom_in_place();
        Ok(())
    }

    fn create_userdata_for_rom(&mut self, filename: &str) -> Result<(), UTF8CString> {
        fn create_if_not_dir(what: &Path) -> Result<(), UTF8CString> {
            if !what.is_dir() && let Err(e) = std::fs::create_dir(what) {
                return Err(format!("Failed to create userdata dir for {}: {e}", what.display()).into());
            }
            Ok(())
        }

        create_if_not_dir(&self.get_userdir_for_rom(filename))?;
        create_if_not_dir(&self.get_save_states_dir_for_rom(filename))?;
        create_if_not_dir(&self.get_save_data_dir_for_rom(filename))?;
        create_if_not_dir(&self.get_replays_dir_for_rom(filename))?;

        Ok(())
    }

    fn get_save_states_dir_for_rom(&self, filename: &str) -> PathBuf {
        self.get_userdir_for_rom(filename).join("save states")
    }

    fn get_save_data_dir_for_rom(&self, filename: &str) -> PathBuf {
        self.get_userdir_for_rom(filename).join("save data")
    }

    fn get_replays_dir_for_rom(&self, filename: &str) -> PathBuf {
        self.get_userdir_for_rom(filename).join("replays")
    }

    fn get_userdir_for_rom(&self, filename: &str) -> PathBuf {
        self.user_dir.join(format!("{filename}-data"))
    }

    fn reload_rom_in_place(&mut self) {
        let emulator_type = self.core_metadata.emulator_type.expect("reload_rom_in_place with no emulator type");
        let rom_name = self.get_current_rom_name().expect("reload_rom_in_place with no loaded ROM");
        let save_file = self.get_current_save_name().expect("reload_rom_in_place with no save file");
        let save_file_data = self.get_save_file_data(rom_name, save_file);
        let rom_data = self.loaded_rom_data.as_ref().map(|i| i.as_slice()).expect("reload_rom_in_place with no loaded rom");
        let core = self.make_new_core(rom_data, save_file_data, emulator_type);
        self.reset_save_state_history();
        self.set_core_in_place(core);
    }

    fn reset_save_state_history(&mut self) {
        self.current_save_state_history = Vec::new();
        self.current_save_state_history_position = 0;
    }

    fn make_new_core(&self, rom_data: &[u8], save_file: Option<Vec<u8>>, emulator_type: SuperShuckieEmulatorType) -> Box<dyn EmulatorCore> {
        let bios = self.get_bios_for_core(emulator_type);

        let mut core: Box<dyn EmulatorCore> = match emulator_type {
            SuperShuckieEmulatorType::GameBoy => Box::new(GameBoyColor::new_from_rom(rom_data, bios.as_slice(), Model::DmgB)),
            SuperShuckieEmulatorType::GameBoyColor => Box::new(GameBoyColor::new_from_rom(rom_data, bios.as_slice(), Model::Cgb0))
        };

        if let Some(sram) = save_file {
            let _ = core.load_sram(sram.as_slice()); // TODO: handle this?
        }

        core
    }

    fn set_core_in_place(&mut self, core: Box<dyn EmulatorCore>) {
        self.before_unload_rom();
        self.core = ThreadedSuperShuckieCore::new(core);
        self.after_switch_core();
        self.after_load_rom();
    }

    #[expect(unused_variables)]
    fn get_current_save_file_name_for_rom(&self, filename: &str) -> UTF8CString {
        // TODO (stub); this should persist for the given ROM
        UTF8CString::from_str("default")
    }

    #[expect(unused_variables)]
    fn get_save_file_data(&self, filename: &str, save_file: &str) -> Option<Vec<u8>> {
        // TODO (stub); this should persist for the given ROM
        None
    }

    fn get_bios_for_core(&self, emulator_kind: SuperShuckieEmulatorType) -> Vec<u8> {
        // TODO: Let this be configurable.
        match emulator_kind {
            SuperShuckieEmulatorType::GameBoy => todo!("DMG BIOS"),
            SuperShuckieEmulatorType::GameBoyColor => include_bytes!("../../bootrom/cgb/cgb_boot/cgb_boot_fast.bin").to_vec()
        }
    }

    pub fn unload_rom(&mut self) {
        self.before_unload_rom();
        self.core = ThreadedSuperShuckieCore::new(Box::new(NullEmulatorCore));
        self.save_file = None;
        self.rom_name = None;
        self.core_metadata.emulator_type = None;
        self.current_input = Input::default();
        self.after_switch_core();
    }

    /// Set whether or not the game is paused.
    pub fn set_paused(&mut self, paused: bool) {
        // we still want to do this for config reasons
        self.settings.emulation.paused = paused;

        if self.is_game_running() {
            if paused {
                self.core.pause();
            }
            else {
                self.core.start();
            }
        }
    }

    /// Save the SRAM.
    ///
    /// No-op if `!self.is_game_running()`
    #[expect(unused_variables)]
    pub fn save_sram(&self) {
        if !self.is_game_running() {
            return
        }

        let current_rom = self.get_current_rom_name().expect("save_sram with no current ROM");
        let current_save = self.get_current_rom_name().expect("save_sram with no current save");

        // TODO
    }

    /// Return `true` if a ROM is running.
    #[inline]
    pub fn is_game_running(&self) -> bool {
        self.core_metadata.emulator_type.is_some()
    }

    /// Calls the `refresh_screens` callback regardless of if there's a new frame.
    #[inline]
    pub fn force_refresh_screens(&mut self) {
        self.refresh_screen(true);
    }

    /// Set the video scale.
    pub fn set_video_scale(&mut self, scale: NonZeroU8) {
        let old_scale = &mut self.settings.emulation.video_scale;
        if scale == *old_scale {
            return
        }

        *old_scale = scale;
        self.update_video_mode();
    }

    /// Set the video scale.
    pub fn set_speed(&mut self, mut base: f64, mut turbo: f64) {
        base = Speed::from_multiplier_float(base).into_multiplier_float();
        turbo = Speed::from_multiplier_float(turbo).into_multiplier_float();

        self.settings.emulation.base_speed_multiplier = base;
        self.settings.emulation.turbo_speed_multiplier = turbo;

        self.apply_turbo(0.0);
    }

    /// Set a custom setting.
    pub fn set_custom_setting(&mut self, setting: &str, value: Option<UTF8CString>) {
        match value {
            Some(n) => { self.settings.custom.insert(setting.to_owned(), n); },
            None => { self.settings.custom.remove(setting); }
        }
    }

    /// Get a custom setting.
    pub fn get_custom_setting(&self, setting: &str) -> Option<&UTF8CString> {
        self.settings.custom.get(setting)
    }

    /// Enqueue an input.
    #[inline]
    pub fn enqueue_input(&mut self, input: Input) {
        self.core.enqueue_input(input);
    }

    /// Handle any logic that needs to be done regularly.
    pub fn tick(&mut self) {
        self.refresh_screen(false);
    }

    fn refresh_screen(&mut self, force: bool) {
        let current_frame_count = self.core.get_frame_count();
        if force || current_frame_count == self.frame_count {
            return
        }

        self.frame_count = current_frame_count;
        self.core.read_screens(|screens| {
            self.callbacks.refresh_screens(screens);
        })
    }

    pub fn get_current_rom_name(&self) -> Option<&str> {
        self.rom_name.as_ref().map(|i| i.as_str())
    }

    pub fn get_current_rom_name_c_str(&self) -> Option<&CStr> {
        self.rom_name.as_ref().map(|i| i.as_c_str())
    }

    pub fn get_current_save_name(&self) -> Option<&str> {
        self.save_file.as_ref().map(|i| i.as_str())
    }

    pub fn get_current_save_name_c_str(&self) -> Option<&CStr> {
        self.save_file.as_ref().map(|i| i.as_c_str())
    }

    pub fn write_settings(&self) {
        // TODO: handle errors here?
        let _ = std::fs::write(self.user_dir.join(SETTINGS_FILE), serde_json::to_string_pretty(&self.settings).expect("failed to serialize"));
    }

    fn before_unload_rom(&mut self) {
        if !self.is_game_running() {
            return
        }

        self.save_sram();

        // probably have to stop replay recording, etc.
    }

    fn after_switch_core(&mut self) {
        self.update_video_mode();
    }

    fn update_video_mode(&mut self) {
        self.core.read_screens(|screens| {
            self.callbacks.change_video_mode(screens, self.settings.emulation.video_scale);
        });
    }

    fn after_load_rom(&mut self) {
        self.force_refresh_screens();
        self.current_input = Input::default();
        self.core.set_speed(Speed::from_multiplier_float(self.settings.emulation.base_speed_multiplier));
        if !self.settings.emulation.paused {
            self.core.start();
        }
    }

    fn apply_turbo(&mut self, turbo: f64) {
        let base_speed = self.settings.emulation.base_speed_multiplier;
        let max_speed = self.settings.emulation.turbo_speed_multiplier * base_speed;
        let total_speed = base_speed + (max_speed - base_speed) * turbo;
        self.core.set_speed(Speed::from_multiplier_float(total_speed));
    }
}

pub struct CoreMetadata {
    pub emulator_type: Option<SuperShuckieEmulatorType>
}

pub trait SuperShuckieFrontendCallbacks {
    fn refresh_screens(&mut self, screens: &[ScreenData]);
    fn change_video_mode(&mut self, screens: &[ScreenData], screen_scaling: NonZeroU8);
}

fn _ensure_callbacks_are_object_safe(_: Box<dyn SuperShuckieFrontendCallbacks>) {}

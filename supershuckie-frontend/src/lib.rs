pub mod util;
pub mod settings;

use crate::settings::*;
use crate::util::UTF8CString;
use std::ffi::CStr;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use supershuckie_core::emulator::{EmulatorCore, GameBoyColor, Input, Model, NullEmulatorCore, ScreenData};
use supershuckie_core::{Speed, ThreadedSuperShuckieCore};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SuperShuckieEmulatorType {
    GameBoy,
    GameBoyColor
}

#[expect(dead_code)]
pub struct SuperShuckieFrontend {
    core: ThreadedSuperShuckieCore,
    core_metadata: CoreMetadata,

    callbacks: Box<dyn SuperShuckieFrontendCallbacks>,

    user_dir: PathBuf,
    frame_count: u32,

    loaded_rom_data: Option<Vec<u8>>,

    current_input: Input,

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
            callbacks,
            settings,
            current_input: Input::default(),
        };

        s.unload_rom();

        s
    }

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_button_input(&mut self, control: &ControlSetting, pressed: bool) {
        if control.control.is_button() {
            match control.modifier {
                ControlModifier::Normal => {
                    control.control.set_for_input(&mut self.current_input, pressed);
                    self.core.enqueue_input(self.current_input);
                },
                ControlModifier::Rapid => {
                    // TODO
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

        self.unload_rom();
        self.loaded_rom_data = Some(data);
        self.rom_name = Some(UTF8CString::from_str(filename));
        self.core_metadata.emulator_type = Some(emulator_to_use);
        self.save_file = Some(self.get_current_save_file_name_for_rom(filename));
        self.reload_rom_in_place();
        Ok(())
    }

    fn reload_rom_in_place(&mut self) {
        let emulator_type = self.core_metadata.emulator_type.expect("reload_rom_in_place with no emulator type");
        let rom_name = self.get_current_rom_name().expect("reload_rom_in_place with no loaded ROM");
        let save_file = self.get_current_save_name().expect("reload_rom_in_place with no save file");
        let save_file_data = self.get_save_file_data(rom_name, save_file);
        let rom_data = self.loaded_rom_data.as_ref().map(|i| i.as_slice()).expect("reload_rom_in_place with no loaded rom");
        let core = self.make_new_core(rom_data, save_file_data, emulator_type);
        self.set_core_in_place(core);
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
        self.core.read_screens(|screens| {
            self.callbacks.new_core_metadata(&self.core_metadata, screens);
        });
    }

    fn after_load_rom(&mut self) {
        self.force_refresh_screens();
        self.current_input = Input::default();
        self.core.set_speed(Speed::from_multiplier_float(self.settings.emulation.base_speed));
        if !self.settings.emulation.paused {
            self.core.start();
        }
    }

    fn apply_turbo(&mut self, turbo: f64) {
        let base_speed = self.settings.emulation.base_speed;
        let max_speed = self.settings.emulation.turbo_speed * base_speed;
        let total_speed = base_speed + (max_speed - base_speed) * turbo;
        self.core.set_speed(Speed::from_multiplier_float(total_speed));
    }
}

pub struct CoreMetadata {
    pub emulator_type: Option<SuperShuckieEmulatorType>
}

pub trait SuperShuckieFrontendCallbacks {
    fn refresh_screens(&mut self, screens: &[ScreenData]);
    fn new_core_metadata(&mut self, core_metadata: &CoreMetadata, screens: &[ScreenData]);
}

fn _ensure_callbacks_are_object_safe(_: Box<dyn SuperShuckieFrontendCallbacks>) {}

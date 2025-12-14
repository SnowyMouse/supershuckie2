pub mod util;

use std::ffi::CStr;
use crate::util::UTF8CString;
use std::path::{Path, PathBuf};
use supershuckie_core::emulator::{EmulatorCore, GameBoyColor, Input, Model, NullEmulatorCore, ScreenData};
use supershuckie_core::ThreadedSuperShuckieCore;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SuperShuckieEmulatorType {
    GameBoy,
    GameBoyColor
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum GBMode {
    /// Run all Game Boy games in Game Boy Color mode
    AlwaysGBC,

    /// Run Game Boy games in Game Boy mode
    GBInGBMode,

    /// Run Game Boy Color games in Game Boy mode if they are backwards compatible
    GBCBackwardsCompatibleInGBMode,

    /// Run all Game Boy games in Game Boy mode, even incompatible Game Boy Color games
    AlwaysGB
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GBSettings {
    pub run_mode: GBMode
}

#[expect(dead_code)]
pub struct SuperShuckieFrontend {
    core: ThreadedSuperShuckieCore,
    core_metadata: CoreMetadata,

    callbacks: Box<dyn SuperShuckieFrontendCallbacks>,

    user_dir: PathBuf,
    paused: bool,
    frame_count: u32,

    loaded_rom_data: Option<Vec<u8>>,

    rom_name: Option<UTF8CString>,
    save_file: Option<UTF8CString>,

    gb_settings: GBSettings
}

impl SuperShuckieFrontend {
    pub fn new<P: AsRef<Path>>(user_dir: P, callbacks: Box<dyn SuperShuckieFrontendCallbacks>) -> Self {
        let mut s = Self {
            core: ThreadedSuperShuckieCore::new(Box::new(NullEmulatorCore)),
            core_metadata: CoreMetadata { emulator_type: None },
            user_dir: user_dir.as_ref().to_owned(),
            rom_name: None,
            save_file: None,
            paused: false,
            loaded_rom_data: None,
            frame_count: 0,
            callbacks,

            // todo: load settings
            gb_settings: GBSettings {
                run_mode: GBMode::AlwaysGBC
            }
        };

        s.unload_rom();

        s
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
                match self.gb_settings.run_mode {
                    GBMode::AlwaysGBC => SuperShuckieEmulatorType::GameBoyColor,
                    GBMode::AlwaysGB => SuperShuckieEmulatorType::GameBoy,

                    // TODO: check header (the extension will not help)
                    GBMode::GBInGBMode => todo!(),
                    GBMode::GBCBackwardsCompatibleInGBMode => todo!()
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
        self.after_switch_core();
    }

    /// Set whether or not the game is paused.
    pub fn set_paused(&mut self, paused: bool) {
        // we still want to do this for config reasons
        self.paused = paused;

        if self.is_game_running() {
            if paused {
                self.core.pause();
            }
            else {
                self.core.start();
            }
        }

        // TODO: persist in config
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
        if !self.paused {
            self.core.start();
        }
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

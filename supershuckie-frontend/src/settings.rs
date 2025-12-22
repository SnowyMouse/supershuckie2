use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs;
use std::fs::File;
use std::hint::unreachable_unchecked;
use std::io::{Read, Seek, SeekFrom};
use std::num::{NonZeroU64, NonZeroU8, NonZeroUsize};
use std::path::Path;
use serde::{Deserialize, Serialize};
use supershuckie_core::emulator::Input;
use supershuckie_replay_recorder::replay_file::record::ReplayFileRecorderSettings;
use crate::SETTINGS_FILE;
use crate::util::UTF8CString;

pub(crate) fn try_to_init_user_dir_and_get_settings(user_dir: &Path) -> Result<Settings, String> {
    if !user_dir.exists() {
        fs::create_dir(&user_dir).map_err(|e| format!("Failed to create the user_dir: {e}"))?;
    }

    let settings_toml = user_dir.join(SETTINGS_FILE);
    let mut settings_file = File::options()
        .write(true)
        .read(true)
        .create(true)
        .open(settings_toml)
        .map_err(|e| format!("Failed to open the settings file for write access: {e}"))?;

    settings_file.seek(SeekFrom::Start(0)).map_err(|e| format!("Failed to seek the settings file: {e}"))?;

    let mut settings_str = String::new();
    settings_file.read_to_string(&mut settings_str).map_err(|e| format!("Failed to read the settings file: {e}"))?;

    if settings_str.trim().is_empty() {
        settings_str = "{}".to_owned();
    }

    let settings: Settings = serde_json::from_str::<Settings>(&settings_str).map_err(|e| format!("Failed to parse the settings file: {e}"))?;
    Ok(settings)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "EmulationSettings::default")]
    pub emulation: EmulationSettings,

    #[serde(default = "GameBoySettings::default")]
    pub game_boy_settings: GameBoySettings,

    #[serde(default = "KeyboardControls::default")]
    pub keyboard_controls: KeyboardControls,
    
    #[serde(default = "ReplaySettings::default")]
    pub replay_settings: ReplaySettings,

    #[serde(default = "BTreeMap::default")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub rom_config: BTreeMap<String, ROMConfig>,

    #[serde(default = "PokeAByteSettings::default")]
    pub pokeabyte: PokeAByteSettings,

    #[serde(default = "BTreeMap::default")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub custom: BTreeMap<String, UTF8CString>
}

impl Settings {
    pub(crate) fn get_rom_config_or_default(&mut self, rom: &str) -> &mut ROMConfig {
        if !self.rom_config.contains_key(rom) {
            self.rom_config.insert(rom.to_owned(), ROMConfig::default());
        }
        self.rom_config.get_mut(rom).expect("we just added the rom??")
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ReplaySettings {
    #[serde(default = "ReplaySettings::DEFAULT_MAX_BLOB_SIZE")]
    pub max_blob_size: NonZeroUsize,

    #[serde(default = "ReplaySettings::DEFAULT_MAX_ZSTD_COMPRESSION_LEVEL")]
    pub zstd_compression_level: i32,

    #[serde(default = "ReplaySettings::DEFAULT_FRAMES_PER_KEYFRAME")]
    pub frames_per_keyframe: NonZeroU64,

    #[serde(default = "ReplaySettings::AUTO_STOP_PLAYBACK_ON_INPUT")]
    pub auto_stop_playback_on_input: bool
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self {
            max_blob_size: Self::DEFAULT_MAX_BLOB_SIZE(),
            zstd_compression_level: Self::DEFAULT_MAX_ZSTD_COMPRESSION_LEVEL(),
            frames_per_keyframe: Self::DEFAULT_FRAMES_PER_KEYFRAME(),
            auto_stop_playback_on_input: Self::AUTO_STOP_PLAYBACK_ON_INPUT()
        }
    }
}

impl ReplaySettings {
    const DEFAULT_MAX_BLOB_SIZE: fn() -> NonZeroUsize = || unsafe { NonZeroUsize::new_unchecked(ReplayFileRecorderSettings::default().minimum_uncompressed_bytes_per_blob) };
    const DEFAULT_MAX_ZSTD_COMPRESSION_LEVEL: fn() -> i32 = || ReplayFileRecorderSettings::default().compression_level;
    const DEFAULT_FRAMES_PER_KEYFRAME: fn() -> NonZeroU64 = || unsafe { NonZeroU64::new_unchecked(60) };
    const AUTO_STOP_PLAYBACK_ON_INPUT: fn() -> bool = || false;
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ROMConfig {
    pub save_name: UTF8CString
}

impl Default for ROMConfig {
    fn default() -> Self {
        Self {
            save_name: "default".into()
        }
    }
}

#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PokeAByteSettings {
    #[serde(default = "bool::default")]
    pub enabled: bool
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct EmulationSettings {
    #[serde(default = "EmulationSettings::DEFAULT_BASE_SPEED_MULTIPLIER")]
    pub base_speed_multiplier: f64,

    #[serde(default = "EmulationSettings::DEFAULT_TURBO_SPEED_MULTIPLIER")]
    pub turbo_speed_multiplier: f64,

    #[serde(default = "EmulationSettings::DEFAULT_VIDEO_SCALE")]
    pub video_scale: NonZeroU8,

    #[serde(default = "EmulationSettings::DEFAULT_PAUSED")]
    pub paused: bool,

    #[serde(default = "EmulationSettings::DEFAULT_MAX_SAVE_STATE_HISTORY")]
    pub max_save_state_history: NonZeroUsize
}

impl EmulationSettings {
    const DEFAULT_BASE_SPEED_MULTIPLIER: fn() -> f64 = || 1.0;
    const DEFAULT_TURBO_SPEED_MULTIPLIER: fn() -> f64 = || 2.0;
    const DEFAULT_VIDEO_SCALE: fn() -> NonZeroU8 = || unsafe { NonZeroU8::new_unchecked(4) };
    const DEFAULT_PAUSED: fn() -> bool = || false;
    const DEFAULT_MAX_SAVE_STATE_HISTORY: fn() -> NonZeroUsize = || unsafe { NonZeroUsize::new_unchecked(100) };
}

impl Default for EmulationSettings {
    fn default() -> Self {
        Self {
            base_speed_multiplier: EmulationSettings::DEFAULT_BASE_SPEED_MULTIPLIER(),
            turbo_speed_multiplier: EmulationSettings::DEFAULT_TURBO_SPEED_MULTIPLIER(),
            video_scale: EmulationSettings::DEFAULT_VIDEO_SCALE(),
            paused: EmulationSettings::DEFAULT_PAUSED(),
            max_save_state_history: EmulationSettings::DEFAULT_MAX_SAVE_STATE_HISTORY()
        }
    }
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameBoySettings {
    #[serde(default = "GameBoyMode::default")]
    pub gbc_mode: GameBoyMode
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum GameBoyMode {
    /// Run all Game Boy games in Game Boy Color mode
    #[serde(rename = "GBC-always")]
    #[default]
    AlwaysGBC,

    /// Run Game Boy games in Game Boy mode
    #[serde(rename = "GBC-auto")]
    GBInGBMode,

    /// Run all Game Boy games in Game Boy mode, even incompatible Game Boy Color games
    #[serde(rename = "GBC-never")]
    AlwaysGB
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyboardControls {
    #[serde(default = "KeyboardControls::DEFAULT_MAP")]
    pub mappings: BTreeMap<i32, ControlSetting>
}

impl KeyboardControls {
    const DEFAULT_MAP: fn() -> BTreeMap<i32, ControlSetting> = || {
        [
            (b'A' as i32, ControlSetting { control: Control::A, modifier: ControlModifier::Normal }),
            (b'S' as i32, ControlSetting { control: Control::B, modifier: ControlModifier::Normal }),
            (b'Z' as i32, ControlSetting { control: Control::A, modifier: ControlModifier::Rapid }),
            (b'X' as i32, ControlSetting { control: Control::B, modifier: ControlModifier::Rapid }),
            (b'D' as i32, ControlSetting { control: Control::X, modifier: ControlModifier::Normal }),
            (b'F' as i32, ControlSetting { control: Control::Y, modifier: ControlModifier::Normal }),
            (b'Q' as i32, ControlSetting { control: Control::L, modifier: ControlModifier::Normal }),
            (b'W' as i32, ControlSetting { control: Control::R, modifier: ControlModifier::Normal }),
            (b' ' as i32, ControlSetting { control: Control::Select, modifier: ControlModifier::Normal }),

            // Return
            (16777220, ControlSetting { control: Control::Start, modifier: ControlModifier::Normal }),

            // Arrow keys
            (16777234, ControlSetting { control: Control::Left, modifier: ControlModifier::Normal }),
            (16777235, ControlSetting { control: Control::Up, modifier: ControlModifier::Normal }),
            (16777236, ControlSetting { control: Control::Right, modifier: ControlModifier::Normal }),
            (16777237, ControlSetting { control: Control::Down, modifier: ControlModifier::Normal }),
        ].into_iter().collect()
    };
}

impl Default for KeyboardControls {
    fn default() -> Self {
        Self {
            mappings: Self::DEFAULT_MAP()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlSetting {
    pub control: Control,
    #[serde(default = "ControlModifier::default")]
    #[serde(skip_serializing_if = "ControlModifier::is_default")]
    pub modifier: ControlModifier
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u32)]
#[serde(rename_all = "snake_case")]
pub enum ControlModifier {
    #[default]
    Normal,
    Rapid,
    Toggle
}

impl ControlModifier {
    fn is_default(&self) -> bool {
        self == &ControlModifier::Normal
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u32)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    A,
    B,
    Start,
    Select,

    Up,
    Down,
    Left,
    Right,

    L,
    R,
    X,
    Y,

    Turbo,
    Reset,
    Pause
}
impl Control {
    pub const fn is_button(self) -> bool {
        match self {
            Control::A => true,
            Control::B => true,
            Control::Start => true,
            Control::Select => true,
            Control::Up => true,
            Control::Down => true,
            Control::Left => true,
            Control::Right => true,
            Control::L => true,
            Control::R => true,
            Control::X => true,
            Control::Y => true,
            Control::Turbo => false,
            Control::Reset => false,
            Control::Pause => false
        }
    }

    pub(crate) const fn set_for_input(&self, input: &mut Input, value: bool) {
        match self {
            Control::A => input.a = value,
            Control::B => input.b = value,
            Control::Start => input.start = value,
            Control::Select => input.select = value,
            Control::Up => input.d_up = value,
            Control::Down => input.d_down = value,
            Control::Left => input.d_left = value,
            Control::Right => input.d_right = value,
            Control::L => input.l = value,
            Control::R => input.r = value,
            Control::X => input.x = value,
            Control::Y => input.y = value,
            Control::Turbo => {}
            Control::Reset => {}
            Control::Pause => {}
        }
    }

    pub(crate) const fn invert_for_input(&self, input: &mut Input) {
        match self {
            Control::A => input.a = !input.a,
            Control::B => input.b = !input.b,
            Control::Start => input.start = !input.start,
            Control::Select => input.select = !input.select,
            Control::Up => input.d_up = !input.d_up,
            Control::Down => input.d_down = !input.d_down,
            Control::Left => input.d_left = !input.d_left,
            Control::Right => input.d_right = !input.d_right,
            Control::L => input.l = !input.l,
            Control::R => input.r = !input.r,
            Control::X => input.x = !input.x,
            Control::Y => input.y = !input.y,
            Control::Turbo => {}
            Control::Reset => {}
            Control::Pause => {}
        }
    }

    #[inline]
    pub const fn as_str(self) -> &'static str {
        let cstr = self.as_c_str();
        let Ok(str) = cstr.to_str() else {
            // SAFETY: Trust me bro.
            unsafe { unreachable_unchecked() }
        };
        str
    }

    pub const fn as_c_str(self) -> &'static CStr {
        match self {
            Control::A => c"A",
            Control::B => c"B",
            Control::Start => c"start",
            Control::Select => c"select",
            Control::Up => c"up",
            Control::Down => c"down",
            Control::Left => c"left",
            Control::Right => c"right",
            Control::L => c"L",
            Control::R => c"R",
            Control::X => c"X",
            Control::Y => c"Y",
            Control::Turbo => c"turbo",
            Control::Reset => c"reset",
            Control::Pause => c"pause"
        }
    }
}

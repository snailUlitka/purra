#![doc = include_str!("../README.md")]

pub mod engine;
pub mod files;
pub mod preset;
pub mod text_engine;
pub mod text_preset;

#[allow(deprecated)]
pub use engine::{Engine, EngineError, Finding, Replacement, Rule};
#[allow(deprecated)]
pub use preset::{Preset, PresetError, PresetLoadError};
pub use text_engine::{TextEngine, TextEngineError, TextFinding, TextRule};
pub use text_preset::{TextPreset, TextPresetError, TextPresetLoadError};

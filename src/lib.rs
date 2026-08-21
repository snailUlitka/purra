#![doc = include_str!("../README.md")]

pub mod engine;
pub mod files;
pub mod preset;

pub use engine::{Engine, EngineError, Finding, Replacement, Rule};
pub use preset::{Preset, PresetError, PresetLoadError};

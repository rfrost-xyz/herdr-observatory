pub mod common;
pub mod model;
pub mod telemetry;
pub type Result<T> = std::result::Result<T, String>;

mod envelope;

pub mod allowances;
pub mod hooks_install;
pub mod identity;
pub mod navigation;
pub mod reporter;

pub mod claude;
pub mod collection;
pub mod config;
pub mod native;
mod turns;

pub mod packaging;

pub mod fleet;

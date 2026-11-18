//! Spike S6 — palette générée depuis `tokens/*.tokens.json`, dégradation et atome `Badge`.
pub mod badge;
pub mod degrade;
pub mod generated;

pub use badge::{Badge, Kind};
pub use degrade::{ColorSupport, degrade};

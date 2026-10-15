use super::Captured;
use anyhow::{bail, Result};

const UNSUPPORTED: &str = "Pegatina solo gestiona el portapapeles en Windows";

pub fn start_listener<F: Fn() + Send + 'static>(_on_change: F) -> Result<()> {
    bail!(UNSUPPORTED)
}

pub fn read_clipboard(_save_passwords: bool) -> Result<Option<Captured>> {
    bail!(UNSUPPORTED)
}

pub fn write_text(_text: &str) -> Result<()> {
    bail!(UNSUPPORTED)
}

pub fn write_image(_png: &[u8]) -> Result<()> {
    bail!(UNSUPPORTED)
}

pub fn write_files(_paths: &[String]) -> Result<()> {
    bail!(UNSUPPORTED)
}

pub fn write_secret(_text: &str) -> Result<()> {
    bail!(UNSUPPORTED)
}

pub fn protect(_plain: &[u8]) -> Result<Vec<u8>> {
    bail!(UNSUPPORTED)
}

pub fn unprotect(_blob: &[u8]) -> Result<Vec<u8>> {
    bail!(UNSUPPORTED)
}

pub fn foreground_window() -> isize {
    0
}

pub fn paste_into(_window: isize) -> Result<()> {
    bail!(UNSUPPORTED)
}

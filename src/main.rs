#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod battle_window;
#[cfg(windows)]
mod collection_window;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
fn main() {
    windows::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows では cargo run、Web では bash scripts/build-web.sh を つかってください。");
}

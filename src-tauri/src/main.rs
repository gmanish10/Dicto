// Prevents a console window from popping up on Windows (we're macOS-only, but harmless).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dicto_lib::run();
}

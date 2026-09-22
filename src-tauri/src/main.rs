// A tray app must not flash a console window on launch (FR-01).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    otr_lib::run()
}

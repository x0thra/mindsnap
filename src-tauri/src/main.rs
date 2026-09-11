// Hide console on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mindsnap_lib::run();
}

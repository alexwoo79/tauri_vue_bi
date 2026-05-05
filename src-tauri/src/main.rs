// src-tauri/src/main.rs
// Prevents additional console window on Windows in release mode — do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri_vue_bi_lib::run();
}

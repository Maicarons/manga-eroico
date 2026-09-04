//! manga-eroico desktop app entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    manga_eroico_lib::run()
}

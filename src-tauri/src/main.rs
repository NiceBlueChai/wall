#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Wall 桌面应用入口。

fn main() {
    wall_lib::run();
}

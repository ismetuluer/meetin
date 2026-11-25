// Sürüm derlemesinde ek konsol penceresini engeller.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    meetin_lib::run();
}

// Unter Windows öffnet die fertige Fassung kein Konsolenfenster neben dem Spiel.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    burrow_boroughs::run();
}

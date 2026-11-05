//! Einstieg für Android: `#[bevy_main]` erzeugt die Funktion, die die GameActivity aufruft.

use bevy::prelude::bevy_main;

#[bevy_main]
fn main() {
    burrow_boroughs::run();
}

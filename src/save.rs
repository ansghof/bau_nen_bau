//! Spielstände: F5 speichert, F9 lädt; zu jeder neuen Jahreszeit wird automatisch gespeichert.
//! Es gibt einen Speicherplatz, eine RON-Datei im Benutzerordner; im Browser liegt sie im localStorage.

use bevy::prelude::*;
use bevy::window::AppLifecycle;
use serde::{Deserialize, Serialize};

use crate::behavior::{Activity, Carrying, Priorities};
use crate::colony::{Colony, Messages, Rng};
use crate::daynight::{DayClock, Season};
use crate::ending::clear_world;
use crate::map::{Grid, Layer, ViewLayer};
use crate::progress::Progress;
use crate::rabbit::{BirthCooldown, Fur, Hunger, Mood, OnLayer, Pregnant, Rabbit, SavedRabbit};
use crate::cat::{CatBond, CatRest};
use crate::scenario::{Losses, Scenario, Stage, chapter};

/// Ändert sich der Aufbau des Spielstands, wird die Zahl erhöht; alte Stände lassen sich dann nicht laden.
const VERSION: u32 = 1;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (save_keys, autosave, save_when_leaving));
    }
}

/// Zum Schreiben: alles nur geliehen.
#[derive(Serialize)]
struct SaveRef<'a> {
    version: u32,
    chapter: u32,
    losses: u32,
    grid: &'a Grid,
    colony: &'a Colony,
    clock: &'a DayClock,
    progress: &'a Progress,
    priorities: &'a Priorities,
    stage: Stage,
    view: Layer,
    rabbits: Vec<SavedRabbit>,
    cat: &'a CatBond,
}

/// Zum Lesen: dieselben Felder, diesmal mit Besitz.
#[derive(Deserialize)]
struct SaveGame {
    version: u32,
    #[serde(default = "first_chapter")]
    chapter: u32,
    #[serde(default)]
    losses: u32,
    grid: Grid,
    colony: Colony,
    clock: DayClock,
    progress: Progress,
    priorities: Priorities,
    stage: Stage,
    view: Layer,
    rabbits: Vec<SavedRabbit>,
    #[serde(default)]
    cat: CatBond,
}

fn first_chapter() -> u32 {
    1
}

const SAVE_FILE: &str = "spielstand.ron";
const CAMPAIGN_FILE: &str = "kampagne.ron";

/// Testläufe schreiben in den Build-Ordner, damit sie keine echten Spielstände überschreiben.
#[cfg(not(target_arch = "wasm32"))]
fn data_path(file: &str) -> std::path::PathBuf {
    use std::path::PathBuf;
    if cfg!(debug_assertions) && std::env::var_os("BUNNY_TEST").is_some() {
        return PathBuf::from("target").join(format!("test-{file}"));
    }
    data_dir().map_or_else(|| PathBuf::from(file), |dir| dir.join(file))
}

/// Der Ordner, in den das Spiel schreiben darf, je nach Betriebssystem.
#[cfg(target_os = "android")]
fn data_dir() -> Option<std::path::PathBuf> {
    bevy::android::ANDROID_APP.get().and_then(|app| app.internal_data_path())
}

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn data_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let env = |name| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|home| home.join(".local/share")))
    };
    let base = base?;
    let dir = base.join("BauNenBau");
    // Das Spiel hieß früher „Bunny Skylines“ und kurz „Burrow Boroughs“:
    // alte Spielstände ziehen einmal mit um.
    for old in ["BurrowBoroughs", "BunnySkylines"].map(|name| base.join(name)) {
        if !dir.exists() && old.is_dir() {
            let _ = std::fs::rename(&old, &dir);
        }
    }
    Some(dir)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_data(file: &str) -> Result<String, String> {
    std::fs::read_to_string(data_path(file)).map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_data(file: &str, text: &str) -> Result<(), String> {
    let path = data_path(file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// Im Browser gibt es keine Dateien; der localStorage merkt sich die Spielstände je Seite.
#[cfg(target_arch = "wasm32")]
fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| "kein localStorage".to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_data(file: &str) -> Result<String, String> {
    let storage = storage()?;
    let get = |prefix: &str| storage.get_item(&format!("{prefix}/{file}")).map_err(|e| format!("{e:?}"));
    // Spielstände aus der Zeit als „Bunny Skylines“ und „Burrow Boroughs“ gelten weiter.
    get("bau_nen_bau")?
        .or(get("burrow_boroughs")?)
        .or(get("bunny_skylines")?)
        .ok_or_else(|| format!("{file} fehlt"))
}

#[cfg(target_arch = "wasm32")]
fn write_data(file: &str, text: &str) -> Result<(), String> {
    storage()?.set_item(&format!("bau_nen_bau/{file}"), text).map_err(|e| format!("{e:?}"))
}

/// Bis zu welchem Kapitel die Kampagne frei ist; das erste ist es immer.
pub fn chapters_unlocked() -> u32 {
    read_data(CAMPAIGN_FILE)
        .ok()
        .and_then(|text| ron::from_str::<u32>(&text).ok())
        .unwrap_or(1)
        .max(1)
}

/// Merkt sich, dass dieses Kapitel jetzt spielbar ist.
pub fn unlock_chapter(number: u32) {
    if number <= chapters_unlocked() {
        return;
    }
    if let Err(error) = write_data(CAMPAIGN_FILE, &number.to_string()) {
        warn!("Kampagnenfortschritt nicht gespeichert: {error}");
    }
}

pub fn save_exists() -> bool {
    read_data(SAVE_FILE).is_ok()
}

fn save_keys(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::F5) {
        commands.run_system_cached(save_game);
    }
    if keys.just_pressed(KeyCode::F9) {
        commands.run_system_cached(load_game);
    }
}

/// Auf dem Handy wird eine App oft einfach weggewischt; vorher wird gespeichert.
fn save_when_leaving(
    mut lifecycle: MessageReader<AppLifecycle>,
    scenario: Res<Scenario>,
    rabbits: Query<(), With<Rabbit>>,
    mut commands: Commands,
) {
    let leaving = lifecycle.read().any(|event| *event == AppLifecycle::WillSuspend);
    if leaving && matches!(scenario.stage, Stage::Playing | Stage::Free) && !rabbits.is_empty() {
        commands.run_system_cached(save_game);
    }
}

/// Zu Beginn jeder Jahreszeit, solange die Familie lebt und gespielt wird.
fn autosave(
    clock: Res<DayClock>,
    scenario: Res<Scenario>,
    rabbits: Query<(), With<Rabbit>>,
    mut commands: Commands,
    mut last: Local<Option<Season>>,
) {
    let season = clock.season();
    let changed = last.is_some_and(|last| last != season);
    *last = Some(season);
    if changed && matches!(scenario.stage, Stage::Playing | Stage::Free) && !rabbits.is_empty() {
        commands.run_system_cached(save_game);
    }
}

pub fn save_game(world: &mut World) {
    let mut query = world.query::<(
        &Rabbit,
        &Fur,
        &Hunger,
        &Mood,
        &OnLayer,
        &Transform,
        &Carrying,
        Option<&Pregnant>,
        Option<&BirthCooldown>,
        &Activity,
    )>();
    let rabbits = query
        .iter(world)
        .filter(|(.., activity)| !matches!(activity, Activity::Gone))
        .map(|(rabbit, fur, hunger, mood, layer, transform, carrying, pregnant, cooldown, _)| {
            SavedRabbit::capture(rabbit, fur, hunger, mood, layer, transform, carrying, pregnant, cooldown)
        })
        .collect();
    // Einleitung und Siegbildschirm sind beim Laden schon vorbei.
    let stage = match world.resource::<Scenario>().stage {
        Stage::Intro | Stage::Playing | Stage::Lost => Stage::Playing,
        Stage::Won | Stage::Free => Stage::Free,
    };
    let save = SaveRef {
        version: VERSION,
        chapter: world.resource::<Scenario>().chapter.number,
        losses: world.resource::<Losses>().0,
        grid: world.resource::<Grid>(),
        colony: world.resource::<Colony>(),
        clock: world.resource::<DayClock>(),
        progress: world.resource::<Progress>(),
        priorities: world.resource::<Priorities>(),
        stage,
        view: world.resource::<ViewLayer>().0,
        rabbits,
        cat: world.resource::<CatBond>(),
    };
    let written = ron::to_string(&save)
        .map_err(|e| e.to_string())
        .and_then(|text| write_data(SAVE_FILE, &text));
    let message = match written {
        Ok(()) => "Spielstand gespeichert.".to_string(),
        Err(error) => {
            warn!("Speichern fehlgeschlagen: {error}");
            "Speichern hat nicht geklappt.".to_string()
        }
    };
    world.resource_mut::<Messages>().push(message);
}

pub fn load_game(world: &mut World) {
    let loaded = read_data(SAVE_FILE)
        .and_then(|text| ron::from_str::<SaveGame>(&text).map_err(|e| e.to_string()));
    let save = match loaded {
        Ok(save) if save.version == VERSION => save,
        Ok(_) => {
            world.resource_mut::<Messages>().push("Der Spielstand stammt von einer älteren Version.");
            return;
        }
        Err(error) => {
            warn!("Laden fehlgeschlagen: {error}");
            world.resource_mut::<Messages>().push("Es gibt noch keinen Spielstand zum Laden.");
            return;
        }
    };

    clear_world(world);
    world.insert_resource(save.grid);
    world.insert_resource(save.colony);
    world.insert_resource(save.clock);
    world.insert_resource(save.progress);
    world.insert_resource(save.priorities);
    world.insert_resource(ViewLayer(save.view));
    world.resource_mut::<Scenario>().stage = save.stage;
    world.resource_mut::<Scenario>().chapter = chapter(save.chapter);
    world.insert_resource(Losses(save.losses));
    world.insert_resource(save.cat);
    world.insert_resource(CatRest::default());
    world.run_system_cached_with(spawn_saved, save.rabbits).ok();
    world.resource_mut::<Messages>().push("Spielstand geladen.");
}

fn spawn_saved(In(rabbits): In<Vec<SavedRabbit>>, mut commands: Commands, asset_server: Res<AssetServer>, mut rng: ResMut<Rng>) {
    for rabbit in rabbits {
        rabbit.spawn(&mut commands, &asset_server, &mut rng);
    }
}

//! Geräusche: Systeme melden mit `Sounds::play`, was passiert ist; hier wird es abgespielt.
//! Dasselbe Geräusch kommt nicht öfter als nötig, damit zwanzig grabende Kaninchen nicht lärmen.
//! Die Dateien liegen unter `assets/sounds/` und stammen vom Generator `tools/sound_gen`.

use std::collections::HashMap;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::rabbit::Stomping;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        // Automatische Testläufe bleiben stumm, sie laufen ja im Hintergrund.
        let muted = cfg!(debug_assertions) && std::env::var_os("BUNNY_TEST").is_some();
        app.insert_resource(Sounds { queue: Vec::new(), muted, music: true })
            .add_systems(Startup, (load_sounds, start_music))
            .add_systems(PostUpdate, ((notice_stomping, play_sounds).chain(), quiet_music));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sound {
    Stomp,
    Alarm,
    Dig,
    Buzzard,
    Fox,
    Birth,
    Research,
    Caught,
    Build,
    Click,
    Victory,
    Extinct,
    Flood,
}

impl Sound {
    const ALL: [Sound; 13] = [
        Sound::Stomp,
        Sound::Alarm,
        Sound::Dig,
        Sound::Buzzard,
        Sound::Fox,
        Sound::Birth,
        Sound::Research,
        Sound::Caught,
        Sound::Build,
        Sound::Click,
        Sound::Victory,
        Sound::Extinct,
        Sound::Flood,
    ];

    fn file(self) -> &'static str {
        match self {
            Sound::Stomp => "stampfen",
            Sound::Alarm => "alarm",
            Sound::Dig => "graben",
            Sound::Buzzard => "bussard",
            Sound::Fox => "fuchs",
            Sound::Birth => "geburt",
            Sound::Research => "erforscht",
            Sound::Caught => "geschnappt",
            Sound::Build => "bauen",
            Sound::Click => "klick",
            Sound::Victory => "sieg",
            Sound::Extinct => "ende",
            Sound::Flood => "wasser",
        }
    }

    fn volume(self) -> f32 {
        match self {
            Sound::Dig => 0.25,
            Sound::Click | Sound::Build => 0.5,
            Sound::Fox => 0.6,
            _ => 0.8,
        }
    }

    /// So viele Sekunden müssen zwischen zwei gleichen Geräuschen liegen.
    fn min_gap(self) -> f32 {
        match self {
            Sound::Dig => 0.6,
            Sound::Stomp => 0.3,
            Sound::Click | Sound::Build => 0.05,
            _ => 0.5,
        }
    }
}

#[derive(Resource)]
pub struct Sounds {
    queue: Vec<Sound>,
    pub muted: bool,
    /// Die Hintergrundmusik lässt sich für sich abschalten; ohne Ton schweigt sie ohnehin.
    pub music: bool,
}

impl Sounds {
    pub fn play(&mut self, sound: Sound) {
        self.queue.push(sound);
    }
}

#[derive(Resource)]
struct SoundFiles(HashMap<Sound, Handle<AudioSource>>);

fn load_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    let files = Sound::ALL
        .into_iter()
        .map(|sound| (sound, asset_server.load(format!("sounds/{}.wav", sound.file()))))
        .collect();
    commands.insert_resource(SoundFiles(files));
}

/// Die Hintergrundmusik, ein Stück aus `tools/sound_gen`, das sich nahtlos wiederholt.
#[derive(Component)]
struct Music;

/// Leise genug, dass Alarm und Fuchs deutlich darüber liegen.
const MUSIC_VOLUME: f32 = 0.35;

fn start_music(mut commands: Commands, asset_server: Res<AssetServer>, sounds: Res<Sounds>) {
    let mut settings = PlaybackSettings::LOOP.with_volume(Volume::Linear(MUSIC_VOLUME));
    if sounds.muted {
        settings = settings.muted();
    }
    commands.spawn((Music, AudioPlayer::new(asset_server.load("sounds/musik.wav")), settings));
}

/// Schaltet man Ton oder Musik ab, verstummt die Musik und spielt später an derselben Stelle weiter.
fn quiet_music(sounds: Res<Sounds>, mut music: Query<&mut AudioSink, With<Music>>) {
    let silent = sounds.muted || !sounds.music;
    for mut sink in &mut music {
        if silent && !sink.is_muted() {
            sink.mute();
        } else if !silent && sink.is_muted() {
            sink.unmute();
        }
    }
}

/// Stampft eine Wache, hört man es.
fn notice_stomping(stomping: Query<(), Added<Stomping>>, mut sounds: ResMut<Sounds>) {
    if !stomping.is_empty() {
        sounds.play(Sound::Stomp);
    }
}

fn play_sounds(
    mut commands: Commands,
    time: Res<Time<Real>>,
    files: Res<SoundFiles>,
    mut sounds: ResMut<Sounds>,
    mut last: Local<HashMap<Sound, f32>>,
) {
    let now = time.elapsed_secs();
    let queue = std::mem::take(&mut sounds.queue);
    if sounds.muted {
        return;
    }
    for sound in queue {
        if last.get(&sound).is_some_and(|at| now - at < sound.min_gap()) {
            continue;
        }
        last.insert(sound, now);
        commands.spawn((
            AudioPlayer::new(files.0[&sound].clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(sound.volume())),
        ));
    }
}

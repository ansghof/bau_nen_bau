//! Die Hintergrundmusik: ein ruhiges Stück in F-Dur, das sich nahtlos wiederholt.
//! Flöte für die Melodie, gezupfte Saiten für die Begleitung, darunter weiche Flächen und ein Bass;
//! im Mittelteil kommt ein leises Rascheln dazu, im letzten Teil ein Glockenspiel.
//! Mono mit halber Abtastrate, damit die Datei auch für die Webfassung klein bleibt.

use std::f32::consts::TAU;

use crate::Noise;

pub const RATE: u32 = 22_050;
/// 80 Schläge pro Minute, vier Schläge pro Takt.
const BEAT: f32 = 0.75;
const BAR: f32 = 4.0 * BEAT;

fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

fn at(seconds: f32) -> usize {
    (seconds * RATE as f32) as usize
}

/// Mischt einen Ton ein; was über das Ende hinausragt, klingt am Anfang weiter,
/// damit die Schleife keine Naht hat.
fn mix_loop(track: &mut [f32], start: f32, sound: &[f32]) {
    let len = track.len();
    let offset = at(start);
    for (i, s) in sound.iter().enumerate() {
        track[(offset + i) % len] += s;
    }
}

/// Flöte: Grundton mit wenigen Obertönen, Vibrato, das erst nach dem Ansatz kommt, und etwas Atem.
fn flute(midi: f32, beats: f32, noise: &mut Noise) -> Vec<f32> {
    let freq = hz(midi);
    let length = beats * BEAT;
    let mut phase = 0.0;
    let mut breath = 0.0;
    (0..at(length + 0.25))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let vibrato = 1.0 + 0.004 * ((t - 0.25).max(0.0) * 4.0).min(1.0) * (t * 5.2 * TAU).sin();
            phase += freq * vibrato / RATE as f32;
            let tone = (phase * TAU).sin() + 0.25 * (2.0 * phase * TAU).sin() + 0.08 * (3.0 * phase * TAU).sin();
            breath += (noise.next() - breath) * 0.2;
            let attack = (t / 0.07).min(1.0);
            let release = ((length + 0.25 - t) / 0.25).clamp(0.0, 1.0);
            let swell = 1.0 - 0.15 * (t / length).min(1.0);
            (tone + breath * 0.12) * attack * release * swell
        })
        .collect()
}

/// Gezupfte Saite: Obertöne, die umso schneller verklingen, je höher sie sind.
/// Additiv statt Karplus-Strong, damit auch hohe Töne bei der halben Abtastrate sauber gestimmt sind.
fn pluck(midi: f32, seconds: f32) -> Vec<f32> {
    let f = hz(midi);
    let harmonics: Vec<(f32, f32, f32)> = (1..=8)
        .map(|k| k as f32)
        .filter(|k| k * f < RATE as f32 * 0.45)
        // Angezupft bei einem Viertel der Saite: so klingen manche Obertöne leiser.
        .map(|k| (k * f, (k * 0.25 * std::f32::consts::PI).sin().abs() / k.powf(1.2), 2.0 + 1.8 * k))
        .collect();
    (0..at(seconds))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let sum: f32 = harmonics.iter().map(|&(fk, amp, decay)| (t * fk * TAU).sin() * amp * (-t * decay).exp()).sum();
            sum * (t / 0.003).min(1.0) * ((seconds - t) / 0.05).clamp(0.0, 1.0)
        })
        .collect()
}

/// Weiche Fläche: leicht verstimmte Sinustöne, die langsam ein- und ausblenden.
fn pad(notes: &[f32], seconds: f32) -> Vec<f32> {
    let fade = 0.8;
    (0..at(seconds + fade))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let level = (t / fade).min(1.0) * ((seconds + fade - t) / fade).clamp(0.0, 1.0);
            let sum: f32 = notes
                .iter()
                .map(|&n| {
                    let f = hz(n);
                    (t * f * TAU).sin() + (t * f * 1.003 * TAU).sin() + 0.3 * (t * 2.0 * f * TAU).sin()
                })
                .sum();
            sum * level
        })
        .collect()
}

/// Bass: runder Sinus mit etwas Oberton, damit man ihn auch auf kleinen Lautsprechern hört.
fn bass(midi: f32, seconds: f32) -> Vec<f32> {
    let f = hz(midi);
    (0..at(seconds))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let level = (t / 0.02).min(1.0) * (-t * 0.9).exp() * ((seconds - t) / 0.1).clamp(0.0, 1.0);
            ((t * f * TAU).sin() + 0.35 * (t * 2.0 * f * TAU).sin()) * level
        })
        .collect()
}

/// Glockenspiel: heller Anschlag mit unharmonischem Oberton, der schnell verklingt.
fn glocken(midi: f32) -> Vec<f32> {
    let f = hz(midi);
    (0..at(1.6))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let body = (t * f * TAU).sin() * (-t * 2.5).exp();
            let shine = 0.4 * (t * f * 2.76 * TAU).sin() * (-t * 7.0).exp();
            (body + shine) * (t / 0.002).min(1.0)
        })
        .collect()
}

/// Ein leises Rascheln wie ein Strohhalm-Shaker.
fn shaker(noise: &mut Noise) -> Vec<f32> {
    let mut low = 0.0;
    (0..at(0.09))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let n = noise.next();
            low += (n - low) * 0.5;
            (n - low) * (t / 0.01).min(1.0) * (-t * 45.0).exp()
        })
        .collect()
}

/// Hall nach Schroeder; läuft zweimal über die Schleife, damit auch der Anfang schon nachhallt.
fn reverb(dry: &[f32]) -> Vec<f32> {
    let mut combs: Vec<(Vec<f32>, usize, f32)> =
        [557, 593, 641, 677].iter().map(|&d| (vec![0.0; d], 0, 0.0)).collect();
    let mut allpasses: Vec<(Vec<f32>, usize)> = [113, 277].iter().map(|&d| (vec![0.0; d], 0)).collect();
    let mut wet = vec![0.0; dry.len()];
    for _pass in 0..2 {
        for (i, &x) in dry.iter().enumerate() {
            let mut sum = 0.0;
            for (buffer, pos, damp) in &mut combs {
                let y = buffer[*pos];
                *damp = y * 0.6 + *damp * 0.4;
                buffer[*pos] = x + *damp * 0.82;
                *pos = (*pos + 1) % buffer.len();
                sum += y;
            }
            let mut y = sum * 0.25;
            for (buffer, pos) in &mut allpasses {
                let delayed = buffer[*pos];
                buffer[*pos] = y + delayed * 0.5;
                y = delayed - y * 0.5;
                *pos = (*pos + 1) % buffer.len();
            }
            wet[i] = y;
        }
    }
    wet
}

/// Akkorde: Grundton des Basses und drei Töne für Fläche und Zupfen.
struct Chord {
    root: f32,
    notes: [f32; 3],
}

const fn chord(root: f32, notes: [f32; 3]) -> Chord {
    Chord { root, notes }
}

const F: Chord = chord(41.0, [53.0, 57.0, 60.0]);
const C: Chord = chord(36.0, [52.0, 55.0, 60.0]);
const DM: Chord = chord(38.0, [53.0, 57.0, 62.0]);
const BB: Chord = chord(46.0, [53.0, 58.0, 62.0]);
const GM: Chord = chord(43.0, [55.0, 58.0, 62.0]);

/// Melodietöne eines Takts: Tonhöhe, Einsatz und Länge in Schlägen.
type Bar = &'static [(f32, f32, f32)];

const HARMONY: [&Chord; 24] = [
    &F, &C, &DM, &BB, &F, &C, &BB, &C, // A
    &DM, &BB, &F, &C, &DM, &BB, &GM, &C, // B
    &F, &C, &DM, &BB, &GM, &C, &F, &C, // A'
];

const MELODY: [Bar; 24] = [
    // A: fragt und antwortet.
    &[(69.0, 1.0, 1.0), (72.0, 2.0, 1.0), (74.0, 3.0, 1.0)],
    &[(72.0, 0.0, 3.0), (67.0, 3.0, 1.0)],
    &[(69.0, 0.0, 1.5), (65.0, 1.5, 0.5), (67.0, 2.0, 2.0)],
    &[(65.0, 0.0, 3.0)],
    &[(69.0, 1.0, 1.0), (72.0, 2.0, 1.0), (77.0, 3.0, 1.0)],
    &[(76.0, 0.0, 1.5), (74.0, 1.5, 0.5), (72.0, 2.0, 2.0)],
    &[(74.0, 0.0, 1.0), (72.0, 1.0, 1.0), (69.0, 2.0, 2.0)],
    &[(67.0, 0.0, 3.0)],
    // B: etwas nachdenklicher.
    &[(74.0, 0.0, 2.0), (72.0, 2.0, 1.0), (69.0, 3.0, 1.0)],
    &[(70.0, 0.0, 2.0), (69.0, 2.0, 1.0), (67.0, 3.0, 1.0)],
    &[(65.0, 0.0, 1.5), (67.0, 1.5, 0.5), (69.0, 2.0, 2.0)],
    &[(67.0, 0.0, 4.0)],
    &[(74.0, 0.0, 1.0), (77.0, 1.0, 1.0), (76.0, 2.0, 1.0), (74.0, 3.0, 1.0)],
    &[(72.0, 0.0, 2.0), (70.0, 2.0, 2.0)],
    &[(69.0, 0.0, 1.0), (67.0, 1.0, 1.0), (70.0, 2.0, 2.0)],
    &[(72.0, 0.0, 4.0)],
    // A': wie am Anfang, dann ein ruhiger Schluss, der wieder nach vorn führt.
    &[(69.0, 1.0, 1.0), (72.0, 2.0, 1.0), (74.0, 3.0, 1.0)],
    &[(72.0, 0.0, 3.0), (67.0, 3.0, 1.0)],
    &[(69.0, 0.0, 1.5), (65.0, 1.5, 0.5), (67.0, 2.0, 2.0)],
    &[(65.0, 0.0, 3.0)],
    &[(70.0, 0.0, 1.0), (69.0, 1.0, 1.0), (67.0, 2.0, 2.0)],
    &[(72.0, 0.0, 1.5), (74.0, 1.5, 0.5), (72.0, 2.0, 2.0)],
    &[(69.0, 0.0, 3.0)],
    &[],
];

/// Welcher Akkordton auf welche Achtel kommt: Grundton, Quinte, Terz darüber, Quinte …
const ARPEGGIO: [usize; 8] = [0, 2, 1, 2, 3, 2, 1, 2];

pub fn music() -> Vec<f32> {
    let bars = HARMONY.len();
    let len = at(bars as f32 * BAR);
    let mut noise = Noise(2024);
    let mut lead = vec![0.0; len];
    let mut band = vec![0.0; len];

    for (b, (harmony, melody)) in HARMONY.iter().zip(MELODY).enumerate() {
        let start = b as f32 * BAR;
        let section = b / 8;

        mix_loop(&mut band, start, &pad(&harmony.notes, BAR).iter().map(|s| s * 0.035).collect::<Vec<_>>());
        for half in 0..2 {
            let bass_note = bass(harmony.root, 2.0 * BEAT);
            let level = if half == 0 { 0.2 } else { 0.12 };
            mix_loop(&mut band, start + half as f32 * 2.0 * BEAT, &bass_note.iter().map(|s| s * level).collect::<Vec<_>>());
        }

        let tones = [harmony.notes[0], harmony.notes[1], harmony.notes[2], harmony.notes[0] + 12.0];
        for (k, &pick) in ARPEGGIO.iter().enumerate() {
            // Die erste Achtel jedes Takts etwas betont.
            let level = if k == 0 { 0.16 } else { 0.11 };
            let string = pluck(tones[pick] + 12.0, 1.4);
            mix_loop(&mut band, start + k as f32 * BEAT / 2.0, &string.iter().map(|s| s * level).collect::<Vec<_>>());
        }

        if section >= 1 {
            for k in 0..8 {
                let level = if k % 2 == 1 { 0.05 } else { 0.025 };
                let shake = shaker(&mut noise);
                mix_loop(&mut band, start + k as f32 * BEAT / 2.0, &shake.iter().map(|s| s * level).collect::<Vec<_>>());
            }
        }

        for &(note, beat, beats) in melody {
            let tone = flute(note, beats, &mut noise);
            mix_loop(&mut lead, start + beat * BEAT, &tone.iter().map(|s| s * 0.16).collect::<Vec<_>>());
            if section == 2 {
                let bell = glocken(note + 12.0);
                mix_loop(&mut lead, start + beat * BEAT, &bell.iter().map(|s| s * 0.06).collect::<Vec<_>>());
            }
        }
    }

    let dry: Vec<f32> = lead.iter().zip(&band).map(|(l, b)| l + b).collect();
    let wet = reverb(&dry);
    let mixed: Vec<f32> = dry.iter().zip(&wet).map(|(d, w)| d * 0.8 + w * 0.45).collect();
    // Leise genug, dass die Geräusche des Spiels darüber gut zu hören sind.
    let peak = mixed.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    mixed.iter().map(|s| s * 0.7 / peak).collect()
}

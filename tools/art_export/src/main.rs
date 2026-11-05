//! Wandelt alle SVG-Dateien unter `art_src/` in PNG-Dateien unter `assets/` um.
//! Die Ordnerstruktur bleibt erhalten: `art_src/figuren/kaninchen/kopf.svg`
//! wird zu `assets/figuren/kaninchen/kopf.png`.
//!
//! Dazu kommen die App-Symbole für Android und die Web-App, ein Ausschnitt aus dem Titelbild.
//!
//! Aufruf aus dem Projektordner: `cargo run -p art_export`

use std::fs;
use std::path::{Path, PathBuf};

use resvg::{tiny_skia, usvg};

fn main() {
    let source = Path::new("art_src");
    let target = Path::new("assets");

    let mut files = Vec::new();
    collect_svgs(source, &mut files);
    if files.is_empty() {
        eprintln!("Keine SVG-Dateien in {} gefunden.", source.display());
        return;
    }

    let mut failed = 0;
    for svg in &files {
        let png = target.join(svg.strip_prefix(source).unwrap()).with_extension("png");
        match export(svg, &png) {
            Ok(()) => println!("{} -> {}", svg.display(), png.display()),
            Err(error) => {
                failed += 1;
                eprintln!("FEHLER {}: {error}", svg.display());
            }
        }
    }
    println!("{} von {} Dateien exportiert.", files.len() - failed, files.len());
    if let Err(error) = app_icons() {
        failed += 1;
        eprintln!("FEHLER App-Symbol: {error}");
    }
    if failed > 0 {
        std::process::exit(1);
    }
}

fn collect_svgs(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_svgs(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "svg") {
            files.push(path);
        }
    }
}

/// Die SVG-Größe ist die PNG-Größe in Pixeln: Teile werden in doppelter Spielgröße gezeichnet.
fn export(svg: &Path, png: &Path) -> Result<(), String> {
    let data = fs::read(svg).map_err(|e| e.to_string())?;
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("SVG hat keine Größe")?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    if let Some(parent) = png.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    pixmap.save_png(png).map_err(|e| e.to_string())
}

/// Der Ausschnitt aus dem Titelbild (1254 × 1254), der das App-Symbol wird: der Kopf des braunen Kaninchens.
const ICON_CROP: (f32, f32, f32) = (100.0, 448.0, 415.0);

/// Wohin welches Symbol in welcher Größe kommt; `true` heißt mit runden Ecken.
const ICONS: &[(&str, u32, bool)] = &[
    ("android/app/src/main/res/mipmap-mdpi/ic_launcher.png", 48, true),
    ("android/app/src/main/res/mipmap-hdpi/ic_launcher.png", 72, true),
    ("android/app/src/main/res/mipmap-xhdpi/ic_launcher.png", 96, true),
    ("android/app/src/main/res/mipmap-xxhdpi/ic_launcher.png", 144, true),
    ("android/app/src/main/res/mipmap-xxxhdpi/ic_launcher.png", 192, true),
    ("web/symbol-192.png", 192, true),
    ("web/symbol-512.png", 512, true),
    // Für Geräte, die das Symbol selbst in Kreis oder Tropfen schneiden: randlos.
    ("web/symbol-maskable-512.png", 512, false),
    ("web/apple-touch-icon.png", 180, false),
];

fn app_icons() -> Result<(), String> {
    let (x, y, size) = ICON_CROP;
    for &(path, pixels, rounded) in ICONS {
        let corner = if rounded { size * 0.18 } else { 0.0 };
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{pixels}" height="{pixels}" viewBox="{x} {y} {size} {size}">
  <clipPath id="ecken"><rect x="{x}" y="{y}" width="{size}" height="{size}" rx="{corner}"/></clipPath>
  <image clip-path="url(#ecken)" x="0" y="0" width="1254" height="1254" xlink:href="titelbild.png"/>
</svg>"#
        );
        let options = usvg::Options { resources_dir: Some(PathBuf::from("assets/ui")), ..usvg::Options::default() };
        let tree = usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())?;
        let mut pixmap = tiny_skia::Pixmap::new(pixels, pixels).ok_or("Symbol hat keine Größe")?;
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
        let png = Path::new(path);
        if let Some(parent) = png.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        pixmap.save_png(png).map_err(|e| e.to_string())?;
        println!("Titelbild -> {path}");
    }
    Ok(())
}

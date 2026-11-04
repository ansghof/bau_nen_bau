//! Bedienung mit den Fingern, für Tablets und Handys.
//! Ein Finger ist die linke Maustaste: Antippen wählt und baut, Ziehen markiert einen Bereich.
//! Zwei Finger bewegen die Kamera: gemeinsam schieben verschiebt die Karte, auseinander und
//! zusammen zoomt, und Drehen der Finger dreht die Karte mit.

use bevy::input::InputSystems;
use bevy::input::touch::Touch;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::camera::{MAX_DISTANCE, MIN_DISTANCE, Orbit, update_cursor};
use crate::tools::Drag;

pub struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchCursor>()
            .add_systems(PreUpdate, finger_as_mouse.after(InputSystems).before(update_cursor));
    }
}

/// Wo der Finger liegt, solange einer den Bildschirm berührt; sonst gilt der Mauszeiger.
#[derive(Resource, Default)]
pub struct TouchCursor(Option<Vec2>);

/// Wohin der Spieler gerade zeigt: mit dem Finger oder mit der Maus.
pub fn pointer(window: &Window, touch: &TouchCursor) -> Option<Vec2> {
    touch.0.or_else(|| window.cursor_position())
}

/// Was die Finger gerade tun.
#[derive(Default)]
struct Gesture {
    /// Ein Finger drückt die linke Maustaste.
    pressing: bool,
    /// Zwei Finger waren auf dem Bildschirm; bis alle weg sind, wird nichts gebaut.
    camera: bool,
}

fn finger_as_mouse(
    touches: Res<Touches>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut cursor: ResMut<TouchCursor>,
    mut orbit: ResMut<Orbit>,
    mut drag: ResMut<Drag>,
    mut gesture: Local<Gesture>,
) {
    let fingers: Vec<&Touch> = touches.iter().collect();
    match fingers.as_slice() {
        [] => {
            // Losgelassen: In diesem Frame zählt noch die Stelle, an der der Finger war.
            cursor.0 = touches.iter_just_released().next().map(|touch| touch.position());
            if gesture.pressing {
                mouse.release(MouseButton::Left);
                gesture.pressing = false;
            }
            gesture.camera = false;
        }
        [finger] => {
            if gesture.camera {
                // Nach dem Zoomen bleibt oft ein Finger kurz liegen; der baut nichts.
                cursor.0 = None;
                return;
            }
            cursor.0 = Some(finger.position());
            if !gesture.pressing {
                mouse.press(MouseButton::Left);
                gesture.pressing = true;
            }
        }
        [a, b, ..] => {
            if gesture.pressing {
                // Der zweite Finger macht aus dem Antippen eine Kamerabewegung: nichts bauen.
                drag.cancel();
                mouse.release(MouseButton::Left);
                gesture.pressing = false;
            }
            gesture.camera = true;
            cursor.0 = None;
            move_camera(a, b, window.height(), &mut orbit);
        }
    }
}

/// Wie die Karte den zwei Fingern folgt.
fn move_camera(a: &Touch, b: &Touch, screen_height: f32, orbit: &mut Orbit) {
    let (before_a, before_b) = (a.previous_position(), b.previous_position());
    let (now_a, now_b) = (a.position(), b.position());

    // Die Karte klebt an den Fingern: schieben sie nach rechts, wandert die Karte mit.
    let shift = (now_a + now_b) / 2.0 - (before_a + before_b) / 2.0;
    let (forward, right) = orbit.ground_axes();
    let scale = orbit.distance / screen_height.max(1.0) * 1.2;
    orbit.focus += (-right * shift.x + forward * shift.y) * scale;

    let (span_before, span_now) = (before_a.distance(before_b), now_a.distance(now_b));
    if span_before > 10.0 && span_now > 10.0 {
        orbit.distance = (orbit.distance * span_before / span_now).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    let angle = |from: Vec2, to: Vec2| (to - from).to_angle();
    let mut turn = angle(now_a, now_b) - angle(before_a, before_b);
    turn = (turn + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    orbit.yaw += turn;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin;
    use bevy::input::touch::{TouchInput, TouchPhase};

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .init_resource::<TouchCursor>()
            .init_resource::<Drag>()
            .insert_resource(Orbit { focus: Vec3::ZERO, yaw: 0.0, pitch: 0.95, distance: 1000.0 })
            .add_systems(PreUpdate, finger_as_mouse.after(InputSystems));
        let window = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
        (app, window)
    }

    fn finger(app: &mut App, window: Entity, id: u64, phase: TouchPhase, position: Vec2) {
        app.world_mut().write_message(TouchInput { phase, position, window, force: None, id });
    }

    fn left(app: &App) -> bool {
        app.world().resource::<ButtonInput<MouseButton>>().pressed(MouseButton::Left)
    }

    #[test]
    fn one_finger_is_the_left_mouse_button() {
        let (mut app, window) = app();
        finger(&mut app, window, 1, TouchPhase::Started, Vec2::new(100.0, 200.0));
        app.update();
        assert!(left(&app));
        assert_eq!(app.world().resource::<TouchCursor>().0, Some(Vec2::new(100.0, 200.0)));

        finger(&mut app, window, 1, TouchPhase::Moved, Vec2::new(150.0, 200.0));
        app.update();
        assert!(left(&app), "Ziehen hält die Taste");
        assert_eq!(app.world().resource::<TouchCursor>().0, Some(Vec2::new(150.0, 200.0)));

        finger(&mut app, window, 1, TouchPhase::Ended, Vec2::new(150.0, 200.0));
        app.update();
        let mouse = app.world().resource::<ButtonInput<MouseButton>>();
        assert!(mouse.just_released(MouseButton::Left));
        // Beim Loslassen zählt noch die Stelle des Fingers, danach wieder die Maus.
        assert_eq!(app.world().resource::<TouchCursor>().0, Some(Vec2::new(150.0, 200.0)));
        app.update();
        assert_eq!(app.world().resource::<TouchCursor>().0, None);
    }

    #[test]
    fn two_fingers_move_the_camera_and_build_nothing() {
        let (mut app, window) = app();
        finger(&mut app, window, 1, TouchPhase::Started, Vec2::new(300.0, 300.0));
        app.update();
        finger(&mut app, window, 2, TouchPhase::Started, Vec2::new(500.0, 300.0));
        app.update();
        assert!(!left(&app), "der zweite Finger bricht das Bauen ab");

        // Spreizen zoomt näher heran.
        finger(&mut app, window, 1, TouchPhase::Moved, Vec2::new(200.0, 300.0));
        finger(&mut app, window, 2, TouchPhase::Moved, Vec2::new(600.0, 300.0));
        app.update();
        let orbit = app.world().resource::<Orbit>();
        assert!(orbit.distance < 1000.0, "näher: {}", orbit.distance);
        assert!(orbit.yaw.abs() < 0.001, "gerade gespreizt dreht nicht");

        // Drehen der Finger dreht die Karte.
        finger(&mut app, window, 1, TouchPhase::Moved, Vec2::new(400.0, 100.0));
        finger(&mut app, window, 2, TouchPhase::Moved, Vec2::new(400.0, 500.0));
        app.update();
        assert!(app.world().resource::<Orbit>().yaw.abs() > 1.0);

        // Bleibt ein Finger liegen, wird er nicht zum Bauen.
        finger(&mut app, window, 2, TouchPhase::Ended, Vec2::new(400.0, 500.0));
        app.update();
        assert!(!left(&app));
        finger(&mut app, window, 1, TouchPhase::Ended, Vec2::new(400.0, 100.0));
        app.update();
        assert!(!left(&app));
    }
}

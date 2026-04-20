mod gl;
mod shaders;

use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use std::time::Duration;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::gl::Renderer;

const BASE_CELL_W: f64 = 10.0;
const BASE_CELL_H: f64 = 16.0;
const TRAIL_MAX: usize = 16;
const HOLD_SPAWN_INTERVAL: f64 = 0.35;

fn viewport_size() -> (f64, f64) {
    web_sys::window()
        .map(|w| {
            let width  = w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1280.0);
            let height = w.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(720.0);
            (width, height)
        })
        .unwrap_or((1280.0, 720.0))
}

fn dims_from_viewport(cell_w: f64, cell_h: f64) -> (usize, usize) {
    let (w, h) = viewport_size();
    let cols = ((w / cell_w) as usize).max(1);
    let rows = ((h / cell_h) as usize).max(1);
    (cols, rows)
}

fn find_canvas() -> Option<HtmlCanvasElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id("wave-canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .ok()
}

fn wheel_delta_y(d: WheelDelta) -> f64 {
    match d {
        WheelDelta::Pixels(p)   => p.y,
        WheelDelta::Lines(p)    => p.y * 20.0,
        WheelDelta::Pages(p)    => p.y * 500.0,
    }
}

#[allow(non_snake_case)]
fn App() -> Element {
    let mut zoom:           Signal<f64>                          = use_signal(|| 1.0);
    let mut dims:           Signal<(usize, usize)>               = use_signal(|| dims_from_viewport(BASE_CELL_W, BASE_CELL_H));
    let mut time:           Signal<f64>                          = use_signal(|| 0.0);
    let mut mouse_grid:     Signal<(f64, f64)>                   = use_signal(|| {
        let (c, r) = dims_from_viewport(BASE_CELL_W, BASE_CELL_H);
        (c as f64 * 0.5, r as f64 * 0.5)
    });
    // (x, y, birth, sign); sign = -1 for right-click void pulses.
    let mut click_pulses:   Signal<Vec<(f64, f64, f64, f64)>>    = use_signal(Vec::new);
    let mut mouse_strength: Signal<f64>                          = use_signal(|| 1.0);
    let mut last_mouse:     Signal<Option<(f64, f64, f64)>>      = use_signal(|| None);
    // Some(sign) while a button is pressed; drives click-and-hold spawns.
    let mut mouse_down:     Signal<Option<f64>>                  = use_signal(|| None);

    // Wire up window resize → dims signal (respects current zoom).
    use_hook(|| {
        let Some(window) = web_sys::window() else { return };
        let closure = Closure::<dyn FnMut()>::new(move || {
            let z = *zoom.peek();
            dims.set(dims_from_viewport(BASE_CELL_W * z, BASE_CELL_H * z));
        });
        window.set_onresize(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    });

    use_future(move || async move {
        let mut renderer: Option<Renderer> = None;
        let mut last_dims: (usize, usize) = (0, 0);
        let mut last_hold_spawn: f64 = -1000.0;
        let mut trail: Vec<(f32, f32, f32)> = Vec::with_capacity(TRAIL_MAX);

        loop {
            gloo_timers::future::sleep(Duration::from_millis(16)).await;

            let t = *time.peek() + 0.016;
            time.set(t);

            click_pulses.with_mut(|p| p.retain(|&(_, _, bt, _)| t - bt < 4.0));

            // Decay strength toward 1.0 (~6%/frame).
            let s = *mouse_strength.peek();
            mouse_strength.set(s * 0.94 + 0.06);

            // Click-and-hold: keep spawning pulses while a button is held.
            if let Some(sign) = *mouse_down.peek() {
                if t - last_hold_spawn >= HOLD_SPAWN_INTERVAL {
                    last_hold_spawn = t;
                    let mg = *mouse_grid.peek();
                    click_pulses.with_mut(|p| {
                        p.push((mg.0, mg.1, t, sign));
                        if p.len() > 10 { p.remove(0); }
                    });
                }
            }

            // Trail: push current mouse position if it moved or enough time passed,
            // then drop anything older than 0.8 s.
            let mg = *mouse_grid.peek();
            let need_push = match trail.last() {
                Some(&(lx, ly, lt)) => {
                    let d = ((mg.0 as f32 - lx).powi(2) + (mg.1 as f32 - ly).powi(2)).sqrt();
                    d > 0.4 || (t as f32 - lt) > 0.04
                }
                None => true,
            };
            if need_push {
                trail.push((mg.0 as f32, mg.1 as f32, t as f32));
                if trail.len() > TRAIL_MAX { trail.remove(0); }
            }
            trail.retain(|&(_, _, bt)| (t as f32 - bt) < 0.8);

            if renderer.is_none() {
                let Some(canvas) = find_canvas() else { continue; };
                renderer = Some(Renderer::new(&canvas, BASE_CELL_W, BASE_CELL_H));
            }
            let Some(ref r) = renderer else { continue; };

            let z = *zoom.peek();
            let cw = BASE_CELL_W * z;
            let ch = BASE_CELL_H * z;

            let (cols, rows) = *dims.peek();
            let canvas_w = cols as f64 * cw;
            let canvas_h = rows as f64 * ch;
            if (cols, rows) != last_dims {
                last_dims = (cols, rows);
                r.set_viewport(canvas_w as i32, canvas_h as i32);
            }

            let pulses_snap: Vec<(f32, f32, f32, f32)> = click_pulses
                .peek()
                .iter()
                .map(|&(x, y, bt, sign)| (x as f32, y as f32, bt as f32, sign as f32))
                .collect();

            r.draw(
                t as f32,
                canvas_w as f32,
                canvas_h as f32,
                cw as f32,
                ch as f32,
                (mg.0 as f32, mg.1 as f32),
                s as f32,
                &pulses_snap,
                &trail,
            );
        }
    });

    let (cols, rows) = dims();
    let z = zoom();
    let cw = BASE_CELL_W * z;
    let ch = BASE_CELL_H * z;
    let canvas_w = cols as f64 * cw;
    let canvas_h = rows as f64 * ch;

    rsx! {
        div {
            style: "
                background: #000;
                margin: 0;
                padding: 0;
                width: 100vw;
                height: 100vh;
                overflow: hidden;
                user-select: none;
                cursor: crosshair;
            ",
            canvas {
                id: "wave-canvas",
                width: "{canvas_w}",
                height: "{canvas_h}",
                style: "display: block;",
                oncontextmenu: move |evt| {
                    evt.prevent_default();
                },
                onwheel: move |evt| {
                    evt.prevent_default();
                    let dy = wheel_delta_y(evt.data().delta());
                    let current = *zoom.peek();
                    let new_zoom = (current * (1.0 - dy * 0.001)).clamp(0.3, 3.0);
                    zoom.set(new_zoom);
                    dims.set(dims_from_viewport(BASE_CELL_W * new_zoom, BASE_CELL_H * new_zoom));
                },
                onmousemove: move |evt| {
                    let c = evt.element_coordinates();
                    let z = *zoom.peek();
                    let cw = BASE_CELL_W * z;
                    let ch = BASE_CELL_H * z;
                    let pos = (c.x / cw, c.y / ch);
                    mouse_grid.set(pos);

                    let t = *time.peek();
                    if let Some((lx, ly, lt)) = *last_mouse.peek() {
                        let dx = pos.0 - lx;
                        let dy = pos.1 - ly;
                        let dt = (t - lt).max(0.001);
                        let speed = (dx * dx + dy * dy).sqrt() / dt;
                        let target = (1.0 + speed * 0.015).min(6.0);
                        let cur = *mouse_strength.peek();
                        mouse_strength.set(cur * 0.6 + target * 0.4);
                    }
                    last_mouse.set(Some((pos.0, pos.1, t)));
                },
                onmousedown: move |evt| {
                    let data = evt.data();
                    let is_right = matches!(data.trigger_button(), Some(MouseButton::Secondary));
                    let sign: f64 = if is_right { -1.0 } else { 1.0 };
                    let c = data.element_coordinates();
                    let z = *zoom.peek();
                    let cw = BASE_CELL_W * z;
                    let ch = BASE_CELL_H * z;
                    let pos = (c.x / cw, c.y / ch);
                    let t = *time.peek();
                    click_pulses.with_mut(|p| {
                        p.push((pos.0, pos.1, t, sign));
                        if p.len() > 10 { p.remove(0); }
                    });
                    mouse_down.set(Some(sign));
                },
                onmouseup: move |_| {
                    mouse_down.set(None);
                },
                onmouseleave: move |_| {
                    mouse_down.set(None);
                },
            }
        }
    }
}

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::default().rootname("main"))
        .launch(App);
}

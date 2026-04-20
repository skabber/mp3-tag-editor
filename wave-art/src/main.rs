mod gl;
mod shaders;

use dioxus::prelude::*;
use std::time::Duration;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::gl::Renderer;

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

#[allow(non_snake_case)]
fn App() -> Element {
    let cell_w: f64 = 10.0;
    let cell_h: f64 = 16.0;

    let mut dims:         Signal<(usize, usize)>       = use_signal(|| dims_from_viewport(cell_w, cell_h));
    let mut time:         Signal<f64>                  = use_signal(|| 0.0);
    let mut mouse_grid:   Signal<(f64, f64)>           = use_signal(|| {
        let (c, r) = dims_from_viewport(cell_w, cell_h);
        (c as f64 * 0.5, r as f64 * 0.5)
    });
    let mut click_pulses: Signal<Vec<(f64, f64, f64)>> = use_signal(Vec::new);

    // Wire up window resize → dims signal. Closure leaks intentionally; it lives
    // for the page's lifetime and the signal handle behind it is 'static.
    use_hook(|| {
        let Some(window) = web_sys::window() else { return };
        let closure = Closure::<dyn FnMut()>::new(move || {
            dims.set(dims_from_viewport(cell_w, cell_h));
        });
        window.set_onresize(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    });

    use_future(move || async move {
        let mut renderer: Option<Renderer> = None;
        let mut last_dims: (usize, usize) = (0, 0);

        loop {
            gloo_timers::future::sleep(Duration::from_millis(16)).await;

            let t = *time.peek() + 0.016;
            time.set(t);

            click_pulses.with_mut(|p| p.retain(|&(_, _, bt)| t - bt < 4.0));

            if renderer.is_none() {
                let Some(canvas) = find_canvas() else { continue; };
                renderer = Some(Renderer::new(&canvas, cell_w, cell_h));
            }
            let Some(ref r) = renderer else { continue; };

            let (cols, rows) = *dims.peek();
            let canvas_w = cols as f64 * cell_w;
            let canvas_h = rows as f64 * cell_h;
            if (cols, rows) != last_dims {
                last_dims = (cols, rows);
                r.set_viewport(canvas_w as i32, canvas_h as i32);
            }

            let mg = *mouse_grid.peek();
            let pulses_snap: Vec<(f32, f32, f32)> = click_pulses
                .peek()
                .iter()
                .map(|&(x, y, bt)| (x as f32, y as f32, bt as f32))
                .collect();

            r.draw(
                t as f32,
                canvas_w as f32,
                canvas_h as f32,
                cell_w as f32,
                cell_h as f32,
                (mg.0 as f32, mg.1 as f32),
                &pulses_snap,
            );
        }
    });

    let (cols, rows) = dims();
    let canvas_w = cols as f64 * cell_w;
    let canvas_h = rows as f64 * cell_h;

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
                onmousemove: move |evt| {
                    let c = evt.element_coordinates();
                    mouse_grid.set((c.x / cell_w, c.y / cell_h));
                },
                onclick: move |evt| {
                    let c = evt.element_coordinates();
                    let t = *time.peek();
                    click_pulses.with_mut(|p| {
                        p.push((c.x / cell_w, c.y / cell_h, t));
                        if p.len() > 10 { p.remove(0); }
                    });
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

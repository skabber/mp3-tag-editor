use dioxus::prelude::*;
use std::fmt::Write as _;
use std::time::Duration;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

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

// ASCII density gradient from sparse to dense
const CHARS: &[char] = &[
    ' ', ' ', ' ', ' ', '.', '.', '\'', '`', ',', ':', ';', '~', '-', '+', '=',
    'i', 'l', 't', 'r', 'f', 'j', 'v', 'c', 'z', 'x', 'n', 'u', 'o', 'e',
    'I', 'T', 'Y', 'J', 'C', 'L', 'F', 'Z', 'V', 'U', 'X',
    '1', '7', 'S', 's', 'a', 'y', 'k', 'h', 'd', 'b', 'q', 'p',
    'O', '0', 'Q', 'D', 'm', 'w', 'g',
    '*', '#', 'M', 'W', 'B', '8', '@',
];

fn base_wave(x: f64, y: f64, t: f64) -> f64 {
    let px = x / 18.0;
    let py = y / 9.0;

    let w1 = (px * 1.0  + t * 0.80).sin();
    let w2 = (py * 1.3  + t * 0.55).sin();
    let w3 = ((px * 0.9 + py * 0.7)  + t * 1.05).sin();
    let w4 = ((px * 0.7 - py * 1.1)  + t * 0.70).cos();
    let cx  = 2.5 + (t * 0.22).sin() * 1.2;
    let cy  = 2.0 + (t * 0.17).cos() * 0.9;
    let d1  = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    let w5  = (d1 * 1.8 - t * 1.40).sin();
    let d2  = ((px - 1.0).powi(2) + (py - 4.0).powi(2)).sqrt();
    let w6  = (d2 * 2.1 - t * 0.95).cos();

    let raw = (w1 + w2 + w3 + w4 * 0.8 + w5 * 0.65 + w6 * 0.5) / 4.75;
    raw * 0.5 + 0.5
}

fn mouse_wave(x: f64, y: f64, t: f64, mx: f64, my: f64) -> f64 {
    let dist = ((x - mx).powi(2) + (y - my).powi(2)).sqrt();
    let ring  = (dist * 0.55 - t * 2.5).sin();
    let decay = (-dist * 0.13).exp();
    ring * decay * 0.55
}

fn click_wave(x: f64, y: f64, t: f64, pulses: &[(f64, f64, f64)]) -> f64 {
    let mut total = 0.0;
    for &(cx, cy, bt) in pulses {
        let age  = t - bt;
        if age < 0.0 || age > 4.0 { continue; }
        let dist    = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        let radius  = age * 11.0;
        let diff    = dist - radius;
        let ring    = (-( diff * diff / 4.0)).exp();
        let fade    = (-age * 0.7).exp();
        total += ring * fade;
    }
    total.clamp(0.0, 1.0)
}

#[inline(always)]
fn lcg_rand(x: usize, y: usize, frame: u64) -> f64 {
    let seed = x
        .wrapping_mul(1_664_525)
        .wrapping_add(y.wrapping_mul(1_013_904_223))
        .wrapping_add((frame as usize).wrapping_mul(22_695_477))
        ^ 0x5851_f42d;
    ((seed >> 16) & 0xFFFF) as f64 / 65535.0
}

fn get_char(wave: f64, x: usize, y: usize, frame: u64) -> char {
    let noise   = lcg_rand(x, y, frame);
    let blended = (wave * 0.68 + noise * 0.32).clamp(0.0, 1.0);
    let idx     = (blended * (CHARS.len() - 1) as f64) as usize;
    CHARS[idx.min(CHARS.len() - 1)]
}

fn hsl_bucket(wave: f64, x: usize, y: usize, t: f64) -> (u16, u8, u8) {
    let base_hue = (t * 22.0) % 360.0;
    let pos_hue  = (x as f64 * 2.3 + y as f64 * 1.7) % 360.0;
    let wave_hue = wave * 140.0;
    let hue      = (base_hue + pos_hue * 0.18 + wave_hue + 360.0) % 360.0;
    let sat      = 65.0 + wave * 35.0;
    let lit      = 12.0 + wave * 58.0;
    // Quantize to reduce distinct fillStyle values
    let h = ((hue / 4.0).round() as u16) % 90;   // 4° steps → 90 buckets
    let s = (sat / 5.0).round() as u8;            // 5% steps
    let l = (lit / 4.0).round() as u8;            // 4% steps
    (h, s, l)
}

fn get_canvas_ctx() -> Option<CanvasRenderingContext2d> {
    let canvas = web_sys::window()?
        .document()?
        .get_element_by_id("wave-canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

#[allow(clippy::too_many_arguments)]
fn draw_frame(
    ctx: &CanvasRenderingContext2d,
    color_buf: &mut String,
    ch_buf: &mut [u8; 4],
    last_color: &mut (u16, u8, u8),
    cols: usize,
    rows: usize,
    t: f64,
    frame: u64,
    mg: (f64, f64),
    pulses: &[(f64, f64, f64)],
    cell_w: f64,
    cell_h: f64,
) {
    let canvas_w = cols as f64 * cell_w;
    let canvas_h = rows as f64 * cell_h;

    ctx.set_fill_style_str("#000");
    ctx.fill_rect(0.0, 0.0, canvas_w, canvas_h);

    // Sentinel that won't match any real bucket — force first set_fill_style.
    *last_color = (u16::MAX, u8::MAX, u8::MAX);

    let baseline_y_offset = cell_h * 0.82; // approximate alphabetic baseline

    for y in 0..rows {
        let py = y as f64 * cell_h + baseline_y_offset;
        for x in 0..cols {
            let bw   = base_wave(x as f64, y as f64, t);
            let mw   = mouse_wave(x as f64, y as f64, t, mg.0, mg.1);
            let cw   = click_wave(x as f64, y as f64, t, pulses);
            let wave = (bw + mw + cw * 0.5).clamp(0.0, 1.0);
            let ch   = get_char(wave, x, y, frame);
            if ch == ' ' { continue; }

            let bucket = hsl_bucket(wave, x, y, t);
            if bucket != *last_color {
                *last_color = bucket;
                let (h, s, l) = bucket;
                color_buf.clear();
                let _ = write!(
                    color_buf,
                    "hsl({},{}%,{}%)",
                    (h as u32) * 4,
                    (s as u32) * 5,
                    (l as u32) * 4,
                );
                ctx.set_fill_style_str(color_buf);
            }

            let px = x as f64 * cell_w;
            let _ = ctx.fill_text(ch.encode_utf8(ch_buf), px, py);
        }
    }
}

fn App() -> Element {
    let cell_w: f64 = 10.0;
    let cell_h: f64 = 16.0;

    let mut dims:         Signal<(usize, usize)>    = use_signal(|| dims_from_viewport(cell_w, cell_h));
    let mut time:         Signal<f64>               = use_signal(|| 0.0);
    let mut mouse_grid:   Signal<(f64, f64)>        = use_signal(|| {
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
        let mut frame: u64 = 0;
        let mut color_buf = String::with_capacity(24);
        let mut ch_buf = [0u8; 4];
        let mut last_color = (u16::MAX, u8::MAX, u8::MAX);
        let mut last_dims = (0usize, 0usize);
        let mut ctx: Option<CanvasRenderingContext2d> = None;

        loop {
            gloo_timers::future::sleep(Duration::from_millis(16)).await;

            let t = *time.peek() + 0.016;
            time.set(t);
            frame = frame.wrapping_add(1);

            click_pulses.with_mut(|p| p.retain(|&(_, _, bt)| t - bt < 4.0));

            if ctx.is_none() {
                ctx = get_canvas_ctx();
            }
            let Some(ref c) = ctx else { continue; };

            // Assigning canvas.width/height (which dioxus does when dims change)
            // clears the 2D context state, so re-apply font/baseline after a resize.
            let current_dims = *dims.peek();
            if current_dims != last_dims {
                last_dims = current_dims;
                c.set_font("13px 'Courier New', Courier, monospace");
                c.set_text_baseline("alphabetic");
            }

            let (cols, rows) = current_dims;
            let mg = *mouse_grid.peek();
            let pulses_snap: Vec<(f64, f64, f64)> = click_pulses.peek().clone();

            draw_frame(
                c,
                &mut color_buf,
                &mut ch_buf,
                &mut last_color,
                cols,
                rows,
                t,
                frame,
                mg,
                &pulses_snap,
                cell_w,
                cell_h,
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

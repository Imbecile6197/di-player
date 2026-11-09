use cairo::{self, Context};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

const BANDS: usize = 48;
const PARTICLES: usize = 36;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VizStyle {
    #[default]
    Bars,
    Wave,
    Sine,
    Circular,
    Particles,
    Mountain,
    Oscilloscope,
    Dots,
}

impl VizStyle {
    pub const ALL: [VizStyle; 8] = [
        VizStyle::Bars,
        VizStyle::Wave,
        VizStyle::Sine,
        VizStyle::Circular,
        VizStyle::Particles,
        VizStyle::Mountain,
        VizStyle::Oscilloscope,
        VizStyle::Dots,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Bars => "bars",
            Self::Wave => "wave",
            Self::Sine => "sine",
            Self::Circular => "circular",
            Self::Particles => "particles",
            Self::Mountain => "mountain",
            Self::Oscilloscope => "oscilloscope",
            Self::Dots => "dots",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "wave" => Self::Wave,
            "sine" => Self::Sine,
            "circular" => Self::Circular,
            "particles" => Self::Particles,
            "mountain" => Self::Mountain,
            "oscilloscope" => Self::Oscilloscope,
            "dots" => Self::Dots,
            _ => Self::Bars,
        }
    }

    pub fn index(self) -> u32 {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .unwrap_or(0) as u32
    }

    pub fn from_index(index: u32) -> Self {
        Self::ALL
            .get(index as usize)
            .copied()
            .unwrap_or(Self::Bars)
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Bars => "viz-bars",
            Self::Wave => "viz-wave",
            Self::Sine => "viz-sine",
            Self::Circular => "viz-circular",
            Self::Particles => "viz-particles",
            Self::Mountain => "viz-mountain",
            Self::Oscilloscope => "viz-oscilloscope",
            Self::Dots => "viz-dots",
        }
    }
}

#[derive(Clone, Copy)]
pub struct VizColors {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Default for VizColors {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.9,
            b: 1.0,
        }
    }
}

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
}

struct VizState {
    bands: [f64; BANDS],
    phase: f64,
    playing: bool,
    style: VizStyle,
    colors: VizColors,
    particles: Vec<Particle>,
    enabled: bool,
}

impl Default for VizState {
    fn default() -> Self {
        let mut particles = Vec::with_capacity(PARTICLES);
        for i in 0..PARTICLES {
            particles.push(Particle {
                x: (i as f64 * 17.0) % 1.0,
                y: (i as f64 * 31.0) % 1.0,
                vx: ((i % 5) as f64 - 2.0) * 0.002,
                vy: -0.004 - ((i % 3) as f64) * 0.001,
                life: (i as f64 * 0.07) % 1.0,
            });
        }
        Self {
            bands: [0.15; BANDS],
            phase: 0.0,
            playing: false,
            style: VizStyle::Bars,
            colors: VizColors::default(),
            particles,
            enabled: true,
        }
    }
}

#[derive(Clone)]
pub struct Visualizer {
    area: DrawingArea,
    state: Rc<RefCell<VizState>>,
}

impl Visualizer {
    pub fn new() -> Self {
        let area = DrawingArea::new();
        area.set_content_height(110);
        area.set_hexpand(true);
        area.add_css_class("di-visualizer");
        area.set_draw_func(|_, cr, w, h| {
            // Placeholder — replaced below once state Rc exists.
            let _ = (cr, w, h);
        });

        let state = Rc::new(RefCell::new(VizState::default()));
        let state_draw = state.clone();
        area.set_draw_func(move |_, cr, width, height| {
            let st = state_draw.borrow();
            if !st.enabled {
                return;
            }
            draw_frame(cr, width as f64, height as f64, &st);
        });

        let viz = Self { area, state };
        viz.start_ticker();
        viz
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    pub fn set_playing(&self, playing: bool) {
        self.state.borrow_mut().playing = playing;
        self.area.queue_draw();
    }

    pub fn set_style(&self, style: VizStyle) {
        self.state.borrow_mut().style = style;
        self.area.queue_draw();
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.state.borrow_mut().enabled = enabled;
        self.area.set_visible(enabled);
        self.area.queue_draw();
    }

    pub fn set_colors(&self, r: f64, g: f64, b: f64) {
        self.state.borrow_mut().colors = VizColors { r, g, b };
        self.area.queue_draw();
    }

    fn start_ticker(&self) {
        let area = self.area.clone();
        let state = self.state.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
            {
                let mut st = state.borrow_mut();
                if !st.enabled {
                    return glib::ControlFlow::Continue;
                }
                tick(&mut st);
            }
            area.queue_draw();
            glib::ControlFlow::Continue
        });
    }
}

fn tick(st: &mut VizState) {
    st.phase += if st.playing { 0.12 } else { 0.02 };
    let energy = if st.playing { 1.0 } else { 0.22 };
    for i in 0..BANDS {
        let t = st.phase + i as f64 * 0.37;
        let noise = pseudo_noise(t) * 0.55 + pseudo_noise(t * 1.7 + 3.1) * 0.35;
        let bass = (1.0 - i as f64 / BANDS as f64).powf(1.4);
        let target = (0.12 + noise * (0.35 + bass * 0.55)) * energy;
        st.bands[i] += (target - st.bands[i]) * 0.28;
    }

    for p in &mut st.particles {
        if st.playing {
            p.x += p.vx * (0.8 + st.bands[(p.life * BANDS as f64) as usize % BANDS] * 2.0);
            p.y += p.vy * (1.0 + energy);
            p.life += 0.012;
            if p.y < -0.05 || p.life > 1.0 {
                p.y = 1.05;
                p.x = (p.x + 0.37) % 1.0;
                p.life = 0.0;
            }
        } else {
            p.life = (p.life + 0.002) % 1.0;
        }
    }
}

fn pseudo_noise(t: f64) -> f64 {
    let s = (t.sin() * 12.9898 + t.cos() * 78.233).sin() * 43758.5453;
    s.fract().abs()
}

fn draw_frame(cr: &Context, w: f64, h: f64, st: &VizState) {
    if w < 4.0 || h < 4.0 {
        return;
    }
    let c = st.colors;
    match st.style {
        VizStyle::Bars => draw_bars(cr, w, h, st, c),
        VizStyle::Wave => draw_wave(cr, w, h, st, c, false),
        VizStyle::Sine => draw_sine(cr, w, h, st, c),
        VizStyle::Circular => draw_circular(cr, w, h, st, c),
        VizStyle::Particles => draw_particles(cr, w, h, st, c),
        VizStyle::Mountain => draw_wave(cr, w, h, st, c, true),
        VizStyle::Oscilloscope => draw_oscilloscope(cr, w, h, st, c),
        VizStyle::Dots => draw_dots(cr, w, h, st, c),
    }
}

fn set_accent(cr: &Context, c: VizColors, a: f64) {
    let _ = cr.set_source_rgba(c.r, c.g, c.b, a);
}

fn draw_bars(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    let gap = 2.0;
    let bar_w = (w / BANDS as f64) - gap;
    for (i, level) in st.bands.iter().enumerate() {
        let x = i as f64 * (bar_w + gap);
        let bh = level.max(0.04) * h * 0.92;
        let y = h - bh;
        let grad = cairo::LinearGradient::new(x, y, x, h);
        let _ = grad.add_color_stop_rgba(0.0, c.r, c.g, c.b, 0.95);
        let _ = grad.add_color_stop_rgba(1.0, c.r, c.g, c.b, 0.15);
        let _ = cr.set_source(&grad);
        cr.rectangle(x, y, bar_w.max(1.0), bh);
        let _ = cr.fill();
    }
}

fn draw_wave(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors, filled: bool) {
    let mid = h * 0.55;
    cr.move_to(0.0, mid);
    for i in 0..BANDS {
        let x = i as f64 / (BANDS - 1) as f64 * w;
        let amp = st.bands[i] * h * 0.45;
        let y = mid - amp;
        cr.line_to(x, y);
    }
    if filled {
        cr.line_to(w, h);
        cr.line_to(0.0, h);
        cr.close_path();
        let grad = cairo::LinearGradient::new(0.0, 0.0, 0.0, h);
        let _ = grad.add_color_stop_rgba(0.0, c.r, c.g, c.b, 0.55);
        let _ = grad.add_color_stop_rgba(1.0, c.r, c.g, c.b, 0.02);
        let _ = cr.set_source(&grad);
        let _ = cr.fill();
        // Mirror mountain
        cr.move_to(0.0, mid);
        for i in 0..BANDS {
            let x = i as f64 / (BANDS - 1) as f64 * w;
            let amp = st.bands[i] * h * 0.35;
            cr.line_to(x, mid + amp);
        }
        set_accent(cr, c, 0.25);
        cr.set_line_width(1.5);
        let _ = cr.stroke();
    } else {
        set_accent(cr, c, 0.9);
        cr.set_line_width(2.2);
        let _ = cr.stroke();
        // Soft mirror
        cr.move_to(0.0, mid);
        for i in 0..BANDS {
            let x = i as f64 / (BANDS - 1) as f64 * w;
            let amp = st.bands[i] * h * 0.25;
            cr.line_to(x, mid + amp);
        }
        set_accent(cr, c, 0.35);
        cr.set_line_width(1.4);
        let _ = cr.stroke();
    }
}

fn draw_sine(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    let mid = h * 0.5;
    for layer in 0..3 {
        let amp_scale = 0.28 - layer as f64 * 0.06;
        let freq = 2.0 + layer as f64;
        let phase = st.phase * (0.7 + layer as f64 * 0.2);
        cr.new_path();
        for i in 0..=64 {
            let t = i as f64 / 64.0;
            let x = t * w;
            let band = st.bands[(t * (BANDS - 1) as f64) as usize];
            let y = mid + ((t * freq * PI * 2.0 + phase).sin()) * h * amp_scale * (0.4 + band);
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        set_accent(cr, c, 0.85 - layer as f64 * 0.22);
        cr.set_line_width(2.0 - layer as f64 * 0.4);
        let _ = cr.stroke();
    }
}

fn draw_circular(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    let cx = w * 0.5;
    let cy = h * 0.5;
    let base = h.min(w) * 0.22;
    for (i, level) in st.bands.iter().enumerate() {
        let a0 = i as f64 / BANDS as f64 * PI * 2.0 + st.phase * 0.15;
        let a1 = (i + 1) as f64 / BANDS as f64 * PI * 2.0 + st.phase * 0.15;
        let r1 = base;
        let r2 = base + level * h * 0.38;
        cr.new_path();
        cr.arc(cx, cy, r2, a0, a1);
        cr.arc_negative(cx, cy, r1, a1, a0);
        cr.close_path();
        set_accent(cr, c, 0.25 + level * 0.7);
        let _ = cr.fill();
    }
    set_accent(cr, c, 0.35);
    cr.arc(cx, cy, base * 0.7, 0.0, PI * 2.0);
    cr.set_line_width(1.5);
    let _ = cr.stroke();
}

fn draw_particles(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    for p in &st.particles {
        let size = 2.0 + (1.0 - p.life) * 4.0;
        set_accent(cr, c, 0.15 + (1.0 - p.life) * 0.75);
        cr.arc(p.x * w, p.y * h, size, 0.0, PI * 2.0);
        let _ = cr.fill();
    }
    // Soft baseline glow
    set_accent(cr, c, 0.12);
    cr.rectangle(0.0, h * 0.85, w, h * 0.15);
    let _ = cr.fill();
}

fn draw_oscilloscope(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    let mid = h * 0.5;
    cr.new_path();
    for i in 0..=96 {
        let t = i as f64 / 96.0;
        let x = t * w;
        let b1 = st.bands[((t * BANDS as f64) as usize) % BANDS];
        let b2 = st.bands[(((t * 1.7) * BANDS as f64) as usize) % BANDS];
        let y = mid
            + (st.phase * 2.0 + t * PI * 4.0).sin() * h * 0.18 * b1
            + (st.phase + t * PI * 7.0).cos() * h * 0.12 * b2;
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    set_accent(cr, c, 0.95);
    cr.set_line_width(1.6);
    let _ = cr.stroke();
}

fn draw_dots(cr: &Context, w: f64, h: f64, st: &VizState, c: VizColors) {
    let cols = 24usize;
    let rows = 6usize;
    let cell_w = w / cols as f64;
    let cell_h = h / rows as f64;
    for row in 0..rows {
        for col in 0..cols {
            let band = st.bands[(col * BANDS / cols) % BANDS];
            let threshold = 1.0 - (row as f64 + 1.0) / rows as f64;
            if band < threshold * 0.85 {
                continue;
            }
            let x = col as f64 * cell_w + cell_w * 0.5;
            let y = row as f64 * cell_h + cell_h * 0.5;
            let r = cell_w.min(cell_h) * 0.28 * (0.5 + band);
            set_accent(cr, c, 0.35 + band * 0.55);
            cr.arc(x, y, r, 0.0, PI * 2.0);
            let _ = cr.fill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viz_style_roundtrip() {
        for style in VizStyle::ALL {
            assert_eq!(VizStyle::from_id(style.id()), style);
            assert_eq!(VizStyle::from_index(style.index()), style);
        }
    }
}

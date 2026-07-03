use macroquad::prelude::*;
use ::rand as rand_crate;
use rand_crate::Rng;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColorTheme {
    Cyberpunk,
    NeonRainbow,
    Volcano,
    Matrix,
}

impl ColorTheme {
    fn get_random_color(&self) -> Color {
        let mut rng = rand_crate::thread_rng();
        match self {
            ColorTheme::Cyberpunk => {
                let rand_val = rng.gen_range(0.0..1.0);
                if rand_val < 0.4 {
                    Color::new(1.0, 0.0, 0.5, 0.8) // Neon Pink/Magenta
                } else if rand_val < 0.8 {
                    Color::new(0.0, 1.0, 1.0, 0.8) // Neon Cyan
                } else {
                    Color::new(0.5, 0.0, 1.0, 0.8) // Electric Purple
                }
            }
            ColorTheme::NeonRainbow => {
                let hue = rng.gen_range(0.0..360.0);
                hsv_to_rgb(hue, 1.0, 1.0)
            }
            ColorTheme::Volcano => {
                let r = rng.gen_range(0.85..1.0);
                let g = rng.gen_range(0.1..0.5);
                let b = rng.gen_range(0.0..0.1);
                Color::new(r, g, b, 0.8)
            }
            ColorTheme::Matrix => {
                let g = rng.gen_range(0.5..1.0);
                Color::new(0.0, g, 0.0, 0.8)
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ColorTheme::Cyberpunk => "Cyberpunk (Magenta/Cyan/Purple)",
            ColorTheme::NeonRainbow => "Neon Rainbow",
            ColorTheme::Volcano => "Volcano (Red/Orange)",
            ColorTheme::Matrix => "Matrix (Digital Green)",
        }
    }

    fn next(&self) -> Self {
        match self {
            ColorTheme::Cyberpunk => ColorTheme::NeonRainbow,
            ColorTheme::NeonRainbow => ColorTheme::Volcano,
            ColorTheme::Volcano => ColorTheme::Matrix,
            ColorTheme::Matrix => ColorTheme::Cyberpunk,
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color::new(r + m, g + m, b + m, 0.85)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RenderMode {
    Particles,
    Liquid,
    SpectrumLines,
    Waveform,
}

impl RenderMode {
    fn name(&self) -> &'static str {
        match self {
            RenderMode::Particles => "Particles (Gravity Flow)",
            RenderMode::Liquid => "Liquid Constellation (Fluid Web)",
            RenderMode::SpectrumLines => "Ring Spectrum (Path Snapping)",
            RenderMode::Waveform => "Oscilloscope Waveform (Flow Wave)",
        }
    }
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    color: Color,
    home_x: f32,
    home_y: f32,
}

impl Particle {
    fn new(width: f32, height: f32, theme: ColorTheme) -> Self {
        let mut rng = rand_crate::thread_rng();
        let rx = rng.gen_range(0.0..width);
        let ry = rng.gen_range(0.0..height);
        Self {
            x: rx,
            y: ry,
            vx: rng.gen_range(-2.0..2.0),
            vy: rng.gen_range(-2.0..2.0),
            size: rng.gen_range(1.5..3.0),
            color: theme.get_random_color(),
            home_x: rx,
            home_y: ry,
        }
    }
}

// Thread-safe audio state using AtomicU32 for float bit sharing
struct SharedAudioState {
    bass: AtomicU32,
    mid: AtomicU32,
    high: AtomicU32,
}

impl SharedAudioState {
    fn new() -> Self {
        Self {
            bass: AtomicU32::new(0.0f32.to_bits()),
            mid: AtomicU32::new(0.0f32.to_bits()),
            high: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    fn set_bass(&self, val: f32) {
        self.bass.store(val.to_bits(), Ordering::Relaxed);
    }
    fn set_mid(&self, val: f32) {
        self.mid.store(val.to_bits(), Ordering::Relaxed);
    }
    fn set_high(&self, val: f32) {
        self.high.store(val.to_bits(), Ordering::Relaxed);
    }

    fn get_bass(&self) -> f32 {
        f32::from_bits(self.bass.load(Ordering::Relaxed))
    }
    fn get_mid(&self) -> f32 {
        f32::from_bits(self.mid.load(Ordering::Relaxed))
    }
    fn get_high(&self) -> f32 {
        f32::from_bits(self.high.load(Ordering::Relaxed))
    }
}

// Lightweight IIR Filter based Audio Analyzer
struct AudioAnalyzer {
    bass_filter: f32,
    high_filter: f32,
    bass_energy: f32,
    mid_energy: f32,
    high_energy: f32,
}

impl AudioAnalyzer {
    fn new() -> Self {
        Self {
            bass_filter: 0.0,
            high_filter: 0.0,
            bass_energy: 0.0,
            mid_energy: 0.0,
            high_energy: 0.0,
        }
    }

    fn process_sample(&mut self, sample: f32) {
        // Low-pass filter for bass (approx. cutoff at 150Hz with 44.1kHz rate, alpha = 0.02)
        let alpha_bass = 0.02f32;
        self.bass_filter += alpha_bass * (sample - self.bass_filter);

        // High-pass filter for treble (approx. cutoff at 4kHz, alpha = 0.4)
        let alpha_high = 0.4f32;
        let high_val = sample - self.bass_filter;
        self.high_filter += alpha_high * (high_val - self.high_filter);

        // Mids: residual signal
        let mid_val = sample - self.bass_filter - self.high_filter;

        // Exponential decay tracking of sample energies
        let decay = 0.96f32;
        self.bass_energy = self.bass_energy * decay + self.bass_filter.abs() * (1.0 - decay);
        self.mid_energy = self.mid_energy * decay + mid_val.abs() * (1.0 - decay);
        self.high_energy = self.high_energy * decay + self.high_filter.abs() * (1.0 - decay);
    }
}

// Audio capture process
fn run_audio_capture(shared_state: Arc<SharedAudioState>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("No default output device found")?;
    let config = device.default_output_config()?;
    
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let mut analyzer = AudioAnalyzer::new();
    let error_callback = |err| eprintln!("Audio stream callback error: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    for &sample in data {
                        analyzer.process_sample(sample);
                    }
                    shared_state.set_bass(analyzer.bass_energy);
                    shared_state.set_mid(analyzer.mid_energy);
                    shared_state.set_high(analyzer.high_energy);
                },
                error_callback,
                None
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    for &sample in data {
                        let f_sample = sample as f32 / i16::MAX as f32;
                        analyzer.process_sample(f_sample);
                    }
                    shared_state.set_bass(analyzer.bass_energy);
                    shared_state.set_mid(analyzer.mid_energy);
                    shared_state.set_high(analyzer.high_energy);
                },
                error_callback,
                None
            )?
        }
        cpal::SampleFormat::U16 => {
            device.build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    for &sample in data {
                        let f_sample = (sample as f32 - i16::MAX as f32) / i16::MAX as f32;
                        analyzer.process_sample(f_sample);
                    }
                    shared_state.set_bass(analyzer.bass_energy);
                    shared_state.set_mid(analyzer.mid_energy);
                    shared_state.set_high(analyzer.high_energy);
                },
                error_callback,
                None
            )?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;
    
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[macroquad::main("Particle Gravity Simulator")]
async fn main() {
    let mut theme = ColorTheme::Cyberpunk;
    let mut render_mode = RenderMode::Particles;
    let mut particle_count = 5000;
    let mut particles = reset_particles(render_mode, 5000, theme);
    
    let mut gravity_strength: f32 = 0.5; // Acceleration factor
    let mut attract_mode = true; // True for pull, False for push
    let mut friction: f32 = 0.98; // Velocity multiplier per frame
    let mut audio_muted = false;
    
    // Setup audio loopback state
    let shared_audio = Arc::new(SharedAudioState::new());
    let audio_state_clone = shared_audio.clone();
    
    std::thread::spawn(move || {
        match run_audio_capture(audio_state_clone) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("[AVISO] No se pudo inicializar la captura de audio loopback: {}.", e);
                eprintln!("La simulación funcionará normalmente sin reactividad de audio.");
            }
        }
    });

    loop {
        // --- INPUT HANDLING ---
        if is_key_pressed(KeyCode::R) {
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            particles = reset_particles(render_mode, active_count, theme);
        }
        
        if is_key_pressed(KeyCode::Space) {
            attract_mode = !attract_mode;
        }
        
        // Cycle visualization mode
        if is_key_pressed(KeyCode::V) {
            render_mode = match render_mode {
                RenderMode::Particles => RenderMode::Liquid,
                RenderMode::Liquid => RenderMode::SpectrumLines,
                RenderMode::SpectrumLines => RenderMode::Waveform,
                RenderMode::Waveform => RenderMode::Particles,
            };
            
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            // Re-initialize particles to distribute them nicely or reset velocities
            particles = reset_particles(render_mode, active_count, theme);
        }
        
        // Toggle audio mute
        if is_key_pressed(KeyCode::M) {
            audio_muted = !audio_muted;
        }
        
        if is_key_pressed(KeyCode::C) {
            theme = theme.next();
            let mut rng = rand_crate::thread_rng();
            for p in &mut particles {
                p.color = theme.get_random_color();
                p.size = rng.gen_range(1.5..3.0);
            }
        }
        
        if is_key_down(KeyCode::Up) {
            gravity_strength = (gravity_strength + 0.05).min(5.0);
        }
        if is_key_down(KeyCode::Down) {
            gravity_strength = (gravity_strength - 0.05).max(0.05);
        }

        if is_key_down(KeyCode::Right) {
            friction = (friction + 0.001).min(1.0);
        }
        if is_key_down(KeyCode::Left) {
            friction = (friction - 0.001).max(0.85);
        }
        
        if is_key_pressed(KeyCode::Key1) {
            particle_count = 1000;
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            particles = reset_particles(render_mode, active_count, theme);
        }
        if is_key_pressed(KeyCode::Key2) {
            particle_count = 3000;
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            particles = reset_particles(render_mode, active_count, theme);
        }
        if is_key_pressed(KeyCode::Key3) {
            particle_count = 5000;
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            particles = reset_particles(render_mode, active_count, theme);
        }
        if is_key_pressed(KeyCode::Key4) {
            particle_count = 10000;
            let active_count = match render_mode {
                RenderMode::SpectrumLines | RenderMode::Waveform => 500,
                _ => particle_count,
            };
            particles = reset_particles(render_mode, active_count, theme);
        }
        
        // --- AUDIO STATE ACQUISITION ---
        let (bass_val, mid_val, high_val) = if audio_muted {
            (0.0, 0.0, 0.0)
        } else {
            let bass = shared_audio.get_bass();
            let mid = shared_audio.get_mid();
            let high = shared_audio.get_high();
            (
                (bass * 500.0).min(30.0),
                (mid * 500.0).min(20.0),
                (high * 500.0).min(20.0),
            )
        };

        // --- PATH SNAP CALCULATIONS (MODES) ---
        let sw = screen_width();
        let sh = screen_height();
        let time = get_time() as f32;
        let p_len = particles.len();
        
        for i in 0..p_len {
            let p = &mut particles[i];
            match render_mode {
                RenderMode::Particles | RenderMode::Liquid => {
                    // Snapping targets aren't used in free modes, keep them at current positions
                    p.home_x = p.x;
                    p.home_y = p.y;
                }
                RenderMode::SpectrumLines => {
                    // Circular orbit visualizer
                    let angle = (i as f32 / p_len as f32) * std::f32::consts::TAU;
                    let base_r = 160.0 + bass_val * 8.0;
                    // A traveling wave that ripples around the circle based on mid frequencies
                    let wave = (angle * 6.0 - time * 6.0).cos() * mid_val * 5.0;
                    // Add treble sparks to the radius
                    let spark = if i % 11 == 0 { high_val * 3.5 } else { 0.0 };
                    
                    let r = base_r + wave + spark;
                    p.home_x = sw / 2.0 + angle.cos() * r;
                    p.home_y = sh / 2.0 + angle.sin() * r;
                }
                RenderMode::Waveform => {
                    // Horizontal oscilloscope sine wave
                    let x_ratio = i as f32 / p_len as f32;
                    p.home_x = x_ratio * sw;
                    let wave1 = (x_ratio * std::f32::consts::TAU * 3.0 + time * 5.0).sin();
                    let wave2 = (x_ratio * std::f32::consts::TAU * 10.0 - time * 10.0).cos() * 0.25;
                    let amp = 30.0 + mid_val * 14.0 + bass_val * 9.0;
                    
                    // High-frequency treble jitter
                    let jitter = if i % 7 == 0 { rand_crate::thread_rng().gen_range(-1.0..1.0) * high_val * 1.5 } else { 0.0 };
                    
                    p.home_y = sh / 2.0 + (wave1 + wave2) * amp + jitter;
                }
            }
        }

        // --- PHYSICS UPDATE ---
        let (mx, my) = mouse_position();
        let mouse_active = is_mouse_button_down(MouseButton::Left) || is_mouse_button_down(MouseButton::Right);
        
        for p in &mut particles {
            // Apply velocity
            p.x += p.vx;
            p.y += p.vy;
            
            // Apply damping
            p.vx *= friction;
            p.vy *= friction;

            // Restoring spring-physics for Ring/Wave path snapping
            match render_mode {
                RenderMode::Particles | RenderMode::Liquid => {
                    // Free floating: drift slowly with the bass if no gravity is active
                    if !mouse_active && bass_val > 0.1 {
                        let mut rng = rand_crate::thread_rng();
                        let drift = bass_val * 0.015;
                        p.vx += rng.gen_range(-drift..drift);
                        p.vy += rng.gen_range(-drift..drift);
                    }
                }
                RenderMode::SpectrumLines | RenderMode::Waveform => {
                    // Snapping physics: F = -k * x
                    // k is small if mouse is active (feels rubbery/stretchy), high if released (snaps back)
                    let k = if mouse_active { 0.005 } else { 0.055 };
                    let dx = p.home_x - p.x;
                    let dy = p.home_y - p.y;
                    
                    p.vx += dx * k;
                    p.vy += dy * k;
                    
                    // Thermal noise to the spring system based on bass
                    if bass_val > 0.05 {
                        let mut rng = rand_crate::thread_rng();
                        let pulse_jitter = bass_val * 0.008;
                        p.vx += rng.gen_range(-pulse_jitter..pulse_jitter);
                        p.vy += rng.gen_range(-pulse_jitter..pulse_jitter);
                    }
                }
            }

            // Audio Effect 1: Mid frequency shake
            if mid_val > 0.05 {
                let mut rng = rand_crate::thread_rng();
                let shake = mid_val * 0.45;
                p.x += rng.gen_range(-shake..shake);
                p.y += rng.gen_range(-shake..shake);
            }

            // Audio Effect 2: High frequency velocity jitter (sparkles)
            if high_val > 0.05 {
                let mut rng = rand_crate::thread_rng();
                let jitter = high_val * 0.12;
                p.vx += rng.gen_range(-jitter..jitter);
                p.vy += rng.gen_range(-jitter..jitter);
            }
            
            // Apply gravity force towards mouse cursor
            if mouse_active {
                let dx = mx - p.x;
                let dy = my - p.y;
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt();
                
                if dist > 2.0 {
                    let dist_soft = dist.max(25.0);
                    
                    // Bass beats add radial outward push (opposing gravity)
                    let bass_push = if bass_val > 0.15 {
                        -bass_val * 0.035
                    } else {
                        0.0
                    };

                    let force = if attract_mode {
                        (gravity_strength / dist_soft) + bass_push
                    } else {
                        -(gravity_strength / dist_soft) + bass_push
                    };
                    
                    p.vx += (dx / dist) * force;
                    p.vy += (dy / dist) * force;
                }
            }
            
            // Handle boundary collisions (disable bounce if smoothly snapping without mouse)
            let bounce = match render_mode {
                RenderMode::Particles | RenderMode::Liquid => true,
                _ => mouse_active,
            };

            if bounce {
                if p.x < 0.0 {
                    p.x = 0.0;
                    p.vx = -p.vx * 0.7;
                } else if p.x > sw {
                    p.x = sw;
                    p.vx = -p.vx * 0.7;
                }
                
                if p.y < 0.0 {
                    p.y = 0.0;
                    p.vy = -p.vy * 0.7;
                } else if p.y > sh {
                    p.y = sh;
                    p.vy = -p.vy * 0.7;
                }
            }
        }
        
        // --- RENDERING ---
        clear_background(Color::new(0.02, 0.02, 0.05, 1.0)); // Dark space blue
        
        // Render constellation lines in Liquid mode
        if render_mode == RenderMode::Liquid {
            let max_dist = 45.0 + bass_val * 2.0; // Lines expand slightly on beats
            let count = p_len.min(1200); // Caps it to 1200 particles to ensure 60fps
            for i in 0..count {
                for j in (i + 1)..count {
                    let pi = &particles[i];
                    let pj = &particles[j];
                    let dx = pi.x - pj.x;
                    let dy = pi.y - pj.y;
                    let dist_sq = dx*dx + dy*dy;
                    if dist_sq < max_dist * max_dist {
                        let dist = dist_sq.sqrt();
                        let alpha = (1.0 - dist / max_dist) * 0.35;
                        let line_color = Color::new(
                            (pi.color.r + pj.color.r) * 0.5,
                            (pi.color.g + pj.color.g) * 0.5,
                            (pi.color.b + pj.color.b) * 0.5,
                            alpha
                        );
                        draw_line(pi.x, pi.y, pj.x, pj.y, 1.0, line_color);
                    }
                }
            }
        }
        
        // Draw gravity pull/push indicator circles around cursor (pulsate with bass)
        if mouse_active {
            let indicator_color = if attract_mode {
                Color::new(0.0, 1.0, 0.5, 0.15 + (bass_val * 0.005).min(0.15))
            } else {
                Color::new(1.0, 0.1, 0.1, 0.15 + (bass_val * 0.005).min(0.15))
            };
            let pulse_radius = 50.0 + gravity_strength * 30.0 + bass_val * 6.0;
            draw_circle(mx, my, pulse_radius, indicator_color);
            draw_circle(mx, my, 3.0 + bass_val * 0.5, if attract_mode { GREEN } else { RED });
        }
        
        // Draw particles or visualizer lines
        match render_mode {
            RenderMode::Particles | RenderMode::Liquid => {
                for p in &particles {
                    draw_circle(p.x, p.y, p.size, p.color);
                }
            }
            RenderMode::SpectrumLines => {
                // Closed circular outline connecting all particles
                let count = particles.len();
                if count > 1 {
                    for i in 0..count {
                        let p1 = &particles[i];
                        let p2 = &particles[(i + 1) % count];
                        draw_line(p1.x, p1.y, p2.x, p2.y, 2.5, p1.color);
                    }
                }
            }
            RenderMode::Waveform => {
                // Continuous line connecting all particles
                let count = particles.len();
                if count > 1 {
                    for i in 0..(count - 1) {
                        let p1 = &particles[i];
                        let p2 = &particles[i + 1];
                        draw_line(p1.x, p1.y, p2.x, p2.y, 2.5, p1.color);
                    }
                }
            }
        }
        
        // Draw GUI Panel overlay (adjusted to 350px tall to fit mode and shortcuts)
        draw_rectangle(10.0, 10.0, 320.0, 350.0, Color::new(0.05, 0.05, 0.1, 0.85));
        draw_rectangle_lines(10.0, 10.0, 320.0, 350.0, 1.5, Color::new(0.3, 0.3, 0.5, 0.5));
        
        let mut y_offset = 30.0;
        let font_size = 18.0;
        
        draw_text("PARTICLE GRAVITY SIMULATOR", 20.0, y_offset, 20.0, SKYBLUE);
        y_offset += 25.0;
        
        draw_text(&format!("Particles: {}", particle_count), 20.0, y_offset, font_size, WHITE);
        y_offset += 20.0;
        
        draw_text(&format!("Theme: {}", theme.name()), 20.0, y_offset, font_size, WHITE);
        y_offset += 20.0;
        
        draw_text("Visual Mode:", 20.0, y_offset, font_size, WHITE);
        y_offset += 16.0;
        draw_text(&format!("  {}", render_mode.name()), 20.0, y_offset, 14.0, YELLOW);
        y_offset += 22.0;

        let mode_str = if attract_mode { "ATTRACT (Pull)" } else { "REPEL (Push)" };
        let mode_color = if attract_mode { GREEN } else { RED };
        draw_text(&format!("Mode: {}", mode_str), 20.0, y_offset, font_size, mode_color);
        y_offset += 20.0;
        
        draw_text(&format!("Gravity: {:.2} (Up/Down)", gravity_strength), 20.0, y_offset, font_size, WHITE);
        y_offset += 20.0;

        draw_text(&format!("Friction/Slide: {:.3} (Left/Right)", 1.0 - friction), 20.0, y_offset, font_size, WHITE);
        y_offset += 25.0;

        // --- AUDIO VISUALIZER HUD BARS ---
        let audio_header = if audio_muted { "Audio: [MUTED] (Key M)" } else { "Audio Spectrogram (Key M)" };
        let header_color = if audio_muted { RED } else { SKYBLUE };
        draw_text(audio_header, 20.0, y_offset, 16.0, header_color);
        y_offset += 18.0;

        let bar_w = 150.0;
        let bar_h = 8.0;

        // Bass
        draw_text("BASS", 20.0, y_offset, 14.0, ORANGE);
        draw_rectangle(75.0, y_offset - 10.0, bar_w, bar_h, Color::new(0.1, 0.1, 0.25, 1.0));
        let bass_ratio = (bass_val / 30.0).min(1.0);
        draw_rectangle(75.0, y_offset - 10.0, bass_ratio * bar_w, bar_h, ORANGE);
        y_offset += 16.0;

        // Mid
        draw_text("MID", 20.0, y_offset, 14.0, GREEN);
        draw_rectangle(75.0, y_offset - 10.0, bar_w, bar_h, Color::new(0.1, 0.1, 0.25, 1.0));
        let mid_ratio = (mid_val / 20.0).min(1.0);
        draw_rectangle(75.0, y_offset - 10.0, mid_ratio * bar_w, bar_h, GREEN);
        y_offset += 16.0;

        // High
        draw_text("TREB", 20.0, y_offset, 14.0, MAGENTA);
        draw_rectangle(75.0, y_offset - 10.0, bar_w, bar_h, Color::new(0.1, 0.1, 0.25, 1.0));
        let high_ratio = (high_val / 20.0).min(1.0);
        draw_rectangle(75.0, y_offset - 10.0, high_ratio * bar_w, bar_h, MAGENTA);
        y_offset += 25.0;
        
        draw_text("Controls:", 20.0, y_offset, font_size, ORANGE);
        y_offset += 18.0;
        draw_text("[Left-Click] Activate Gravity at cursor", 20.0, y_offset, 15.0, GRAY);
        y_offset += 15.0;
        draw_text("[Space] Toggle Gravity Mode  [C] Color Theme", 20.0, y_offset, 15.0, GRAY);
        y_offset += 15.0;
        draw_text("[V] Cycle Visual Mode        [M] Toggle Audio Mute", 20.0, y_offset, 15.0, GRAY);
        y_offset += 15.0;
        draw_text("[R] Reset Particles          [1-4] Set Count", 20.0, y_offset, 15.0, GRAY);
        
        next_frame().await
    }
}

fn reset_particles(render_mode: RenderMode, count: usize, theme: ColorTheme) -> Vec<Particle> {
    let mut particles = init_particles(count, theme);
    let sw = screen_width();
    let sh = screen_height();
    let width = if sw > 0.0 { sw } else { 800.0 };
    let height = if sh > 0.0 { sh } else { 600.0 };
    
    let p_len = particles.len();
    for i in 0..p_len {
        let p = &mut particles[i];
        match render_mode {
            RenderMode::SpectrumLines => {
                let angle = (i as f32 / p_len as f32) * std::f32::consts::TAU;
                let r = 160.0;
                p.x = width / 2.0 + angle.cos() * r;
                p.y = height / 2.0 + angle.sin() * r;
                p.vx = 0.0;
                p.vy = 0.0;
            }
            RenderMode::Waveform => {
                let x_ratio = i as f32 / p_len as f32;
                p.x = x_ratio * width;
                p.y = height / 2.0;
                p.vx = 0.0;
                p.vy = 0.0;
            }
            _ => {}
        }
    }
    particles
}

fn init_particles(count: usize, theme: ColorTheme) -> Vec<Particle> {
    let sw = screen_width();
    let sh = screen_height();
    
    let width = if sw > 0.0 { sw } else { 800.0 };
    let height = if sh > 0.0 { sh } else { 600.0 };
    
    let mut particles = Vec::with_capacity(count);
    for _ in 0..count {
        particles.push(Particle::new(width, height, theme));
    }
    particles
}

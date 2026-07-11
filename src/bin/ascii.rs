use clap::Parser;
use image::{GenericImageView, Pixel, Rgb, RgbImage};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "ascii",
    version = "0.1.0",
    about = "Converts images to ASCII art with truecolor terminal support"
)]
struct Args {
    /// Path to the input image file (not needed when using --six)
    input: Option<String>,

    /// Target width in terminal columns (characters)
    #[arg(short, long, default_value_t = 100)]
    width: u32,

    /// Target height in terminal rows (lines). If omitted, calculates based on aspect ratio.
    #[arg(long)]
    height: Option<u32>,

    /// Display in monochrome (disable ANSI terminal colors)
    #[arg(short, long)]
    mono: bool,

    /// Invert the character brightness mapping (useful for light/dark terminal backgrounds)
    #[arg(short, long)]
    invert: bool,

    /// Delay in milliseconds between lines to animate construction (static mode only)
    #[arg(short, long, default_value_t = 30)]
    delay: u64,

    /// Rotate the image 360 degrees in place, looping until Ctrl+C is pressed
    #[arg(short, long)]
    rotate: bool,

    /// Number of frames for one full 360-degree rotation (rotate mode only)
    #[arg(long, default_value_t = 60)]
    frames: u32,

    /// Delay in milliseconds between animation frames. Defaults to 50ms for --rotate and
    /// 1500ms for --six.
    #[arg(long)]
    frame_delay: Option<u64>,

    /// Play an image sequence from a directory in order, looping until Ctrl+C is pressed.
    /// Defaults to `assets/six` when passed with no value.
    #[arg(long, num_args = 0..=1, default_missing_value = "assets/six")]
    six: Option<String>,
}

/// Renders a single row of an image as ASCII/ANSI-colored text.
fn render_ascii_line(
    img: &impl GenericImageView<Pixel = image::Rgb<u8>>,
    y: u32,
    width: u32,
    ramp: &[char],
    mono: bool,
) -> String {
    let mut line = String::with_capacity(width as usize * 12);
    // Track the last emitted color so consecutive same-colored pixels reuse one escape
    // code instead of resetting per character — cuts frame size drastically and reduces
    // terminal repaint work (a major source of flicker during animation).
    let mut last_color: Option<[u8; 3]> = None;
    for x in 0..width {
        let pixel = img.get_pixel(x, y);
        let rgb = pixel.to_rgb();
        let [r, g, b] = rgb.0;

        // Calculate perceived brightness using BT.709 weights
        let brightness = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;

        // Map brightness (0.0 to 255.0) to index in character ramp
        let ramp_idx = ((brightness / 255.0) * (ramp.len() - 1) as f32).round() as usize;
        let ch = ramp[ramp_idx.min(ramp.len() - 1)];

        if !mono {
            if last_color != Some([r, g, b]) {
                line.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                last_color = Some([r, g, b]);
            }
            line.push(ch);
        } else {
            line.push(ch);
        }
    }
    if !mono && last_color.is_some() {
        line.push_str("\x1b[0m");
    }
    line
}

/// Renders a full frame (all rows) as a single owned string, one line per row.
fn render_ascii_frame(
    img: &impl GenericImageView<Pixel = image::Rgb<u8>>,
    width: u32,
    height: u32,
    ramp: &[char],
    mono: bool,
) -> Vec<String> {
    (0..height)
        .map(|y| render_ascii_line(img, y, width, ramp, mono))
        .collect()
}

/// Rotates `source` by `angle_radians` about its center and resamples directly into a
/// `target_width x target_height` image, combining rotation and downscaling in one pass.
/// Destination pixels that map outside the source bounds are filled with `background`.
fn rotate_and_sample(
    source: &RgbImage,
    angle_radians: f32,
    target_width: u32,
    target_height: u32,
    background: Rgb<u8>,
) -> RgbImage {
    let (src_w, src_h) = source.dimensions();
    let (src_cx, src_cy) = (src_w as f32 / 2.0, src_h as f32 / 2.0);
    let (dst_cx, dst_cy) = (target_width as f32 / 2.0, target_height as f32 / 2.0);

    // Scale so the destination grid samples proportionally from the source canvas.
    let scale_x = src_w as f32 / target_width as f32;
    let scale_y = src_h as f32 / target_height as f32;

    // Inverse rotation: rotate destination coordinates by -angle to find the source sample.
    let cos_a = angle_radians.cos();
    let sin_a = angle_radians.sin();

    let mut out = RgbImage::new(target_width, target_height);
    for y in 0..target_height {
        for x in 0..target_width {
            let dx = (x as f32 - dst_cx) * scale_x;
            let dy = (y as f32 - dst_cy) * scale_y;

            let src_x = dx * cos_a + dy * sin_a + src_cx;
            let src_y = -dx * sin_a + dy * cos_a + src_cy;

            let pixel = if src_x >= 0.0 && src_y >= 0.0 && (src_x as u32) < src_w && (src_y as u32) < src_h {
                *source.get_pixel(src_x as u32, src_y as u32)
            } else {
                background
            };
            out.put_pixel(x, y, pixel);
        }
    }
    out
}

/// Best-effort RAII guard that restores terminal cursor visibility on drop.
struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        print!("\x1b[?25h");
        let _ = stdout().flush();
    }
}

/// Common file extensions accepted when scanning a directory for an image sequence.
const SEQUENCE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];

/// Reads all image files in `dir`, sorted by filename (so zero-padded numbering like
/// `frame-001.jpg`, `frame-002.jpg`, ... plays back in the right order).
fn load_sequence_dir(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if !dir.is_dir() {
        return Err(format!("Error: '{}' is not a directory.", dir.display()).into());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| SEQUENCE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("Error: no image files found in '{}'.", dir.display()).into());
    }
    Ok(paths)
}

/// Plays back pre-rendered ASCII frames in a loop, redrawing in place until Ctrl+C is
/// pressed. Shared by both the rotation animation and the image-sequence playback mode.
fn run_terminal_animation(frames: &[Vec<String>], frame_delay: u64) {
    if frames.is_empty() {
        return;
    }
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = Arc::clone(&running);
        let _ = ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        });
    }

    let mut out = stdout();
    print!("\x1b[?25l"); // hide cursor
    print!("\x1b[2J\x1b[H"); // clear screen, move cursor home
    let _guard = CursorGuard;

    let mut i: usize = 0;
    while running.load(Ordering::SeqCst) {
        let lines = &frames[i % frames.len()];

        let mut frame = String::with_capacity(lines.iter().map(|l| l.len() + 8).sum());
        // Synchronized update: tells supporting terminals (Windows Terminal, VS Code,
        // iTerm2, kitty...) to buffer this whole frame and paint it in one pass instead
        // of repainting as bytes arrive, which is the main remaining source of flicker.
        // Ignored harmlessly by terminals that don't support it.
        frame.push_str("\x1b[?2026h");
        frame.push_str("\x1b[H");
        for line in lines {
            frame.push_str(line);
            frame.push_str("\x1b[K\n");
        }
        frame.push_str("\x1b[?2026l");

        if out.write_all(frame.as_bytes()).is_err() || out.flush().is_err() {
            break;
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(frame_delay));
        i = i.wrapping_add(1);
    }
    // `_guard` drops here, restoring the cursor.
}

/// Derives the target ASCII grid size from the source image's dimensions and the CLI args.
fn compute_target_dims(orig_width: u32, orig_height: u32, args: &Args) -> (u32, u32) {
    let target_width = args.width.max(1);
    let target_height = match args.height {
        Some(h) => h.max(1),
        None => {
            // Terminal fonts are usually ~2 times taller than they are wide.
            // A vertical scale correction factor of 0.55 helps retain the correct aspect ratio.
            let aspect_ratio = orig_height as f32 / orig_width as f32;
            let corrected_height = target_width as f32 * aspect_ratio * 0.55;
            (corrected_height.round() as u32).max(1)
        }
    };
    (target_width, target_height)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let _ = enable_ansi_support::enable_ansi_support();

    let args = Args::parse();

    // Standard character ramp from darkest to brightest
    let mut ramp = vec![' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    if args.invert {
        ramp.reverse();
    }

    if let Some(dir) = &args.six {
        let dir = Path::new(dir);
        let paths = load_sequence_dir(dir)?;

        let first = image::ImageReader::open(&paths[0])?
            .with_guessed_format()?
            .decode()?;
        let (target_width, target_height) = compute_target_dims(first.width(), first.height(), &args);

        println!(
            "Loading {} frames from {}...",
            paths.len(),
            dir.display()
        );
        let mut frames = Vec::with_capacity(paths.len());
        for p in &paths {
            let frame_img = image::ImageReader::open(p)?
                .with_guessed_format()?
                .decode()?
                .resize_exact(target_width, target_height, image::imageops::FilterType::Triangle)
                .to_rgb8();
            frames.push(render_ascii_frame(&frame_img, target_width, target_height, &ramp, args.mono));
        }

        println!(
            "Playing {} frames ({}x{} chars)... Press Ctrl+C to stop.",
            frames.len(),
            target_width,
            target_height
        );
        std::thread::sleep(std::time::Duration::from_millis(500));

        run_terminal_animation(&frames, args.frame_delay.unwrap_or(1500));
        return Ok(());
    }

    let input = args
        .input
        .clone()
        .ok_or("Error: an input image path is required (or use --six).")?;

    // Check if file exists
    let path = Path::new(&input);
    if !path.exists() {
        return Err(format!("Error: File '{}' does not exist.", input).into());
    }

    println!("Loading image: {}...", input);
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;
    let (orig_width, orig_height) = img.dimensions();

    // Calculate dimensions
    let (target_width, target_height) = compute_target_dims(orig_width, orig_height, &args);

    if args.rotate {
        // Resize once to a square intermediate canvas so the image can rotate in place
        // without the sampled area running out of source pixels. Capped to keep per-frame
        // sampling cost bounded regardless of the source image's original resolution.
        let canvas_dim = (target_width.max(target_height) * 2).min(400);
        let canvas = img
            .resize_to_fill(canvas_dim, canvas_dim, image::imageops::FilterType::Triangle)
            .to_rgb8();
        let background = Rgb([0, 0, 0]);
        let frame_count = args.frames.max(1);

        println!(
            "Rotating {} ({}x{} chars)... Press Ctrl+C to stop.",
            input, target_width, target_height
        );
        std::thread::sleep(std::time::Duration::from_millis(500));

        let frames: Vec<Vec<String>> = (0..frame_count)
            .map(|i| {
                let angle = i as f32 / frame_count as f32 * 2.0 * std::f32::consts::PI;
                let rotated = rotate_and_sample(&canvas, angle, target_width, target_height, background);
                render_ascii_frame(&rotated, target_width, target_height, &ramp, args.mono)
            })
            .collect();

        run_terminal_animation(&frames, args.frame_delay.unwrap_or(50));
        return Ok(());
    }

    println!("Resizing from {}x{} to {}x{}...", orig_width, orig_height, target_width, target_height);
    let resized_img = img
        .resize_exact(target_width, target_height, image::imageops::FilterType::Triangle)
        .to_rgb8();

    println!("--- ASCII ART START ---");
    for y in 0..target_height {
        let line = render_ascii_line(&resized_img, y, target_width, &ramp, args.mono);
        println!("{}", line);
        if args.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(args.delay));
        }
    }
    println!("--- ASCII ART END ---");

    Ok(())
}

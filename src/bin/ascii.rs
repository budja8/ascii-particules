use clap::Parser;
use image::{GenericImageView, Pixel};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "ascii",
    version = "0.1.0",
    about = "Converts images to ASCII art with truecolor terminal support"
)]
struct Args {
    /// Path to the input image file
    input: String,

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

    /// Delay in milliseconds between lines to animate construction
    #[arg(short, long, default_value_t = 30)]
    delay: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let _ = enable_ansi_support::enable_ansi_support();

    let args = Args::parse();

    // Check if file exists
    let path = Path::new(&args.input);
    if !path.exists() {
        return Err(format!("Error: File '{}' does not exist.", args.input).into());
    }

    println!("Loading image: {}...", args.input);
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;
    let (orig_width, orig_height) = img.dimensions();

    // Calculate dimensions
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

    println!("Resizing from {}x{} to {}x{}...", orig_width, orig_height, target_width, target_height);
    let resized_img = img.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Triangle,
    );

    // Standard character ramp from darkest to brightest
    let mut ramp = vec![' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
    if args.invert {
        ramp.reverse();
    }

    println!("--- ASCII ART START ---");
    for y in 0..target_height {
        let mut line = String::with_capacity(target_width as usize * 20); // Pre-allocate memory for speed
        for x in 0..target_width {
            let pixel = resized_img.get_pixel(x, y);
            let rgb = pixel.to_rgb();
            let [r, g, b] = rgb.0;

            // Calculate perceived brightness using BT.709 weights
            let brightness = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
            
            // Map brightness (0.0 to 255.0) to index in character ramp
            let ramp_idx = ((brightness / 255.0) * (ramp.len() - 1) as f32).round() as usize;
            let ch = ramp[ramp_idx.min(ramp.len() - 1)];

            if !args.mono {
                // ANSI 24-bit Truecolor escape code: \x1b[38;2;r;g;bm
                line.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, ch));
            } else {
                line.push(ch);
            }
        }
        println!("{}", line);
        if args.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(args.delay));
        }
    }
    println!("--- ASCII ART END ---");

    Ok(())
}

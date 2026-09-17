use image::ImageReader;
use std::env;
use std::path::PathBuf;

const THUMB_W: u32 = 200;
const THUMB_H: u32 = 200;
const EXTS: [&str; 4] = ["jpg", "jpeg", "png", "gif"];

fn main() {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/tw".into());
    let source = PathBuf::from(&home).join("Wallpapers");
    let cache = PathBuf::from(&home).join(".cache/thumbnails/hdots/wall_thumbs");

    if std::fs::create_dir_all(&cache).is_err() {
        eprintln!("failed to create cache dir: {}", cache.display());
        std::process::exit(1);
    }

    println!("Scanning: {}", source.display());

    let mut generated = 0u32;
    let mut failed = 0u32;

    let dir = match std::fs::read_dir(&source) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot read source dir {}: {}", source.display(), e);
            std::process::exit(1);
        }
    };

    for ent in dir.flatten() {
        let name = match ent.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !EXTS.contains(&ext.as_str()) {
            continue;
        }

        let out_path = cache.join(&name);
        if out_path.is_file() {
            continue; // already generated
        }

        println!("Processing: {}", name);
        match create_thumb(&ent.path(), &out_path) {
            Ok(true) => {
                generated += 1;
                println!("Generated: {}", out_path.display());
            }
            Ok(false) | Err(_) => {
                failed += 1;
                println!("Failed to process: {}", name);
            }
        }
    }

    println!("Done. {} generated, {} failed.", generated, failed);
}

fn create_thumb(input: &std::path::Path, output: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
    let img = match ImageReader::open(input)?.decode() {
        Ok(img) => img,
        Err(e) => {
            eprintln!("cannot decode {}: {}", input.display(), e);
            return Ok(false);
        }
    };
    let thumb = image::imageops::resize(&img.to_rgb8(), THUMB_W, THUMB_H, image::imageops::FilterType::Triangle);
    thumb.save(output)?;
    Ok(true)
}

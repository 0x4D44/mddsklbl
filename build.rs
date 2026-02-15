#[cfg(windows)]
fn make_icon() -> std::path::PathBuf {
    use ico::{IconDir, IconDirEntry, IconImage};
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ico_path = out_dir.join("app.ico");

    let mut dir = IconDir::new(ico::ResourceType::Icon);
    for &size in &[16u32, 32, 48, 64, 128, 256] {
        let img = render_icon(size);
        let rgba = img.into_raw();
        let icon_image = IconImage::from_rgba_data(size, size, rgba);
        let entry = IconDirEntry::encode(&icon_image).expect("encode icon entry");
        dir.add_entry(entry);
    }
    let mut file = fs::File::create(&ico_path).expect("create ico");
    dir.write(&mut file).expect("write ico");
    ico_path
}

/// Signed distance to a rounded rectangle centered at origin with half-extents (hw, hh)
/// and corner radius r. Negative = inside, positive = outside.
fn sd_rounded_rect(px: f32, py: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = px.abs() - hw + r;
    let qy = py.abs() - hh + r;
    let outside = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

/// Render the icon at a given size. Design: a dark slate background with a white border,
/// a bright horizontal "label bar" (pill shape) in the upper third, and faint content
/// lines below — representing the desktop overlay concept.
#[cfg(windows)]
fn render_icon(size: u32) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    use image::{ImageBuffer, Rgba};

    let s = size as f32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(size, size);

    // Dimensions relative to icon size
    let bg_radius = s * 0.18; // background corner radius
    let border_w = (s * 0.06).max(1.0); // white border thickness
    let cx = s / 2.0; // center x
    let cy = s / 2.0; // center y
    let half = s / 2.0 - 0.5; // half-extent of the background square

    // Label bar: a bright horizontal pill in the upper portion
    let bar_cy = cy - s * 0.18; // bar center y (above center)
    let bar_hw = s * 0.34; // bar half-width
    let bar_hh = s * 0.08; // bar half-height
    let bar_r = bar_hh; // fully rounded ends (pill)

    // Content lines below the bar
    let line_h = (s * 0.03).max(0.5); // line thickness
    let line_gap = s * 0.12; // spacing between lines

    // Gradient top/bottom for the background
    let bg_top = [34u8, 46, 72];
    let bg_bot = [16u8, 22, 36];

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            // Distance from pixel to background rounded rect
            let d_bg = sd_rounded_rect(px - cx, py - cy, half, half, bg_radius);

            if d_bg > 1.0 {
                // Outside: transparent
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }

            // Background fill with vertical gradient
            let t = (py / s).clamp(0.0, 1.0);
            let lerp = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 };
            let mut cr = lerp(bg_top[0], bg_bot[0]);
            let mut cg = lerp(bg_top[1], bg_bot[1]);
            let mut cb = lerp(bg_top[2], bg_bot[2]);
            let mut ca = 255u8;

            // Anti-aliased edge for the background
            if d_bg > -1.0 {
                let edge_alpha = ((-d_bg + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                ca = edge_alpha;
            }

            // White border: draw inside the background near the edge
            if d_bg > -border_w - 1.0 && d_bg < 0.5 {
                let border_blend = ((d_bg + border_w + 0.5) / 1.0).clamp(0.0, 1.0);
                // Smooth inner edge of the border
                let inner_blend = ((-d_bg) / 1.0).clamp(0.0, 1.0);
                let b_alpha = (border_blend * inner_blend).clamp(0.0, 1.0);
                // Blend white over the background
                cr = ((1.0 - b_alpha) * cr as f32 + b_alpha * 220.0) as u8;
                cg = ((1.0 - b_alpha) * cg as f32 + b_alpha * 225.0) as u8;
                cb = ((1.0 - b_alpha) * cb as f32 + b_alpha * 235.0) as u8;
            }

            // Label bar (pill shape) — bright accent
            let d_bar = sd_rounded_rect(px - cx, py - bar_cy, bar_hw, bar_hh, bar_r);
            if d_bar < 1.0 {
                let bar_alpha = ((-d_bar + 0.5) * 1.0).clamp(0.0, 1.0);
                // Horizontal gradient on the bar: brighter in center, slightly dimmer at edges
                let bx = ((px - cx) / bar_hw).abs();
                let brightness = 1.0 - bx * 0.15;
                let br = (90.0 * brightness) as u8;
                let bbg = (170.0 * brightness) as u8;
                let bb = (255.0 * brightness) as u8;
                cr = ((1.0 - bar_alpha) * cr as f32 + bar_alpha * br as f32) as u8;
                cg = ((1.0 - bar_alpha) * cg as f32 + bar_alpha * bbg as f32) as u8;
                cb = ((1.0 - bar_alpha) * cb as f32 + bar_alpha * bb as f32) as u8;
            }

            // Content lines below the bar (suggest text on the desktop)
            for i in 0..3 {
                let line_cy = bar_cy + bar_hh + line_gap * (i as f32 + 1.0);
                // Each successive line is shorter
                let line_w = bar_hw * (1.0 - 0.18 * i as f32);
                let dy = (py - line_cy).abs() - line_h;
                let dx = (px - cx).abs() - line_w;
                let d_line = dx.max(dy);
                if d_line < 1.0 && d_bg < -border_w {
                    // Fade lines further from the bar
                    let fade = 1.0 - (i as f32 * 0.3);
                    let line_alpha = ((-d_line + 0.5) * fade).clamp(0.0, 1.0) * 0.45;
                    cr = ((1.0 - line_alpha) * cr as f32 + line_alpha * 180.0) as u8;
                    cg = ((1.0 - line_alpha) * cg as f32 + line_alpha * 195.0) as u8;
                    cb = ((1.0 - line_alpha) * cb as f32 + line_alpha * 220.0) as u8;
                }
            }

            img.put_pixel(x, y, Rgba([cr, cg, cb, ca]));
        }
    }
    img
}

fn main() {
    // Generate an icon at build time and compile Windows version resources (icon + version info).
    #[cfg(windows)]
    {
        // Embed a manifest enabling Per-Monitor v2 DPI awareness.
        embed_manifest::embed_manifest_file("app.manifest").expect("failed to embed manifest file");
        let ico_path = make_icon();
        let mut res = winres::WindowsResource::new();
        res.set_icon(&ico_path.to_string_lossy());

        // Version/info resources
        let pkg_ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "1.0.0".into());
        let file_ver = format!("{pkg_ver}.0"); // Windows expects 4-part versions
        res.set("FileDescription", "Desktop Labeler");
        res.set("ProductName", "Desktop Labeler");
        res.set("CompanyName", "0x4D44 Software");
        res.set("FileVersion", &file_ver);
        res.set("ProductVersion", &file_ver);
        res.set("InternalName", "mddsklbl");
        res.set("OriginalFilename", "mddsklbl.exe");
        res.set("Comments", "Repo: mddsklbl");
        res.set("LegalCopyright", "(C) 2025-2026 0x4D44 Software");

        res.compile().expect("failed to compile Windows resources");
    }
}

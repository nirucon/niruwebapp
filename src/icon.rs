//! Webapp icons: a local letter icon in the NIRUX accent colour, an
//! explicit opt-in favicon fetch, and a minimal PNG encoder.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use std::path::Path;
use std::process::{Command, Stdio};

pub const SIZE: u32 = 128;

// ---------------------------------------------------------------- colours

/// Accent colours of the built-in NIRUWM themes (same table as nirulauncher).
fn builtin_accent(theme: &str) -> Option<u32> {
    Some(match theme {
        "niru-noir" => 0x5a5a60,
        "niru-nord" => 0x88c0d0,
        "niru-satie" => 0xb08d7d,
        "niru-carl-larsson" => 0xa63d2f,
        "niru-forest-vigil" => 0x8aa87e,
        "niru-blood-of-urdr" => 0xc96050,
        "niru-root-and-stone" => 0xc19768,
        "niru-blue-horizon" => 0x86aec8,
        "niru-the-norns" => 0xc9c2b9,
        "niru-winter-nights" => 0xd8c4aa,
        _ => return None,
    })
}

/// `key = "value"` inside `[section]` (empty section = top level).
fn toml_string(text: &str, section: &str, key: &str) -> Option<String> {
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            current = line.trim_matches(['[', ']']).trim().to_string();
            continue;
        }
        if current != section || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() == key {
            let v = v.split('#').next().unwrap_or("").trim();
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}

fn hex(value: &str) -> Option<u32> {
    let value = value.trim().trim_start_matches('#');
    (value.len() == 6)
        .then(|| u32::from_str_radix(value, 16).ok())
        .flatten()
}

/// The NIRUX accent: the selected theme (built-in or user file), with the
/// appearance contract's colour overrides when no theme file exists.
pub fn accent(config_home: &Path) -> u32 {
    let base = config_home.join("niruwm");
    let theme = std::fs::read_to_string(base.join("config.toml"))
        .ok()
        .and_then(|text| toml_string(&text, "", "theme"))
        .unwrap_or_else(|| "niru-noir".into());
    let mut colour = builtin_accent(&theme).unwrap_or(0x5a5a60);
    let theme_file = base.join("themes").join(format!("{theme}.toml"));
    if let Ok(text) = std::fs::read_to_string(&theme_file) {
        for key in ["focus", "accent"] {
            if let Some(c) = toml_string(&text, "", key).and_then(|v| hex(&v)) {
                colour = c;
            }
        }
    } else if let Ok(text) = std::fs::read_to_string(base.join("appearance.toml")) {
        for key in ["focus", "accent"] {
            if let Some(c) = toml_string(&text, "colors", key).and_then(|v| hex(&v)) {
                colour = c;
            }
        }
    }
    colour
}

fn luminance(rgb: u32) -> f32 {
    let channel = |shift: u32| ((rgb >> shift) & 0xff) as f32 / 255.0;
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

// ---------------------------------------------------------------- letter

/// The character shown on a letter icon.
pub fn letter_for(name: &str) -> char {
    name.chars()
        .find(|ch| ch.is_alphanumeric())
        .and_then(|ch| ch.to_uppercase().next())
        .unwrap_or('?')
}

/// The label on a letter icon: the first letter, or the first two when
/// another webapp starts with the same letter (ChatGPT, Claude and Copilot
/// would otherwise share one icon).
pub fn label_for<'a>(name: &str, others: impl IntoIterator<Item = &'a str>) -> String {
    let first = letter_for(name);
    if !others.into_iter().any(|other| letter_for(other) == first) {
        return first.to_string();
    }
    let second = name
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .nth(1)
        .and_then(|ch| ch.to_lowercase().next());
    match second {
        Some(second) => format!("{first}{second}"),
        None => first.to_string(),
    }
}

const FONT_PATHS: [&str; 7] = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    "/usr/share/fonts/noto/NotoSans-Bold.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Bold.ttf",
];

fn read_font(path: &Path) -> Option<FontVec> {
    FontVec::try_from_vec(std::fs::read(path).ok()?).ok()
}

/// A bold sans font: well-known paths first, then fontconfig.
fn load_font() -> Option<FontVec> {
    FONT_PATHS
        .iter()
        .find_map(|path| read_font(Path::new(path)))
        .or_else(|| {
            let output = Command::new("fc-match")
                .args(["-f", "%{file}", "sans-serif:bold"])
                .stderr(Stdio::null())
                .output()
                .ok()?;
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (!path.is_empty())
                .then(|| read_font(Path::new(&path)))
                .flatten()
        })
}

/// 5×7 fallback glyphs when no TrueType font is installed.
fn bitmap_glyph(letter: char) -> [u8; 7] {
    match letter {
        'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        'C' => [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        'G' => [0x0e, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0f],
        'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'I' => [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0c],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        '3' => [0x1f, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
        '6' => [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
        _ => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
    }
}

/// Coverage (0..1) of the label on a SIZE×SIZE canvas, centred.
fn label_coverage(label: &str, font: Option<&FontVec>) -> Vec<f32> {
    let n = SIZE as usize;
    let mut coverage = vec![0f32; n * n];
    let count = label.chars().count().max(1);
    if let Some(font) = font {
        let scale = PxScale::from(if count == 1 { 84.0 } else { 64.0 });
        let scaled = font.as_scaled(scale);
        let mut x = 0.0;
        let mut outlines = Vec::new();
        for ch in label.chars() {
            let glyph = scaled
                .glyph_id(ch)
                .with_scale_and_position(scale, ab_glyph::point(x, 0.0));
            x += scaled.h_advance(glyph.id);
            outlines.extend(font.outline_glyph(glyph));
        }
        if !outlines.is_empty() {
            let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
            let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
            for outline in &outlines {
                let b = outline.px_bounds();
                min_x = min_x.min(b.min.x);
                min_y = min_y.min(b.min.y);
                max_x = max_x.max(b.max.x);
                max_y = max_y.max(b.max.y);
            }
            let dx = ((SIZE as f32 - (max_x - min_x)) / 2.0 - min_x).round() as i32;
            let dy = ((SIZE as f32 - (max_y - min_y)) / 2.0 - min_y).round() as i32;
            for outline in &outlines {
                let b = outline.px_bounds();
                outline.draw(|gx, gy, c| {
                    let px = gx as i32 + b.min.x as i32 + dx;
                    let py = gy as i32 + b.min.y as i32 + dy;
                    if (0..n as i32).contains(&px) && (0..n as i32).contains(&py) {
                        let cell = &mut coverage[py as usize * n + px as usize];
                        *cell = (*cell + c).clamp(0.0, 1.0);
                    }
                });
            }
            return coverage;
        }
    }
    // Bitmap fallback: 5×7 cells, uppercase only.
    let cell = if count == 1 { 10usize } else { 8 };
    let glyphs: Vec<[u8; 7]> = label
        .chars()
        .map(|ch| ch.to_ascii_uppercase())
        .map(|ch| bitmap_glyph(if ch.is_ascii() { ch } else { '?' }))
        .collect();
    let width = glyphs.len() * 6 * cell - cell;
    let (ox, oy) = ((n - width) / 2, (n - 7 * cell) / 2);
    for (index, rows) in glyphs.iter().enumerate() {
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (0x10 >> col) != 0 {
                    let x0 = ox + index * 6 * cell + col * cell;
                    for y in 0..cell {
                        for x in 0..cell {
                            coverage[(oy + row * cell + y) * n + x0 + x] = 1.0;
                        }
                    }
                }
            }
        }
    }
    coverage
}

/// Alpha of a rounded square filling the canvas with a small margin,
/// 4×4 supersampled.
fn shape_alpha(x: u32, y: u32) -> f32 {
    let (margin, radius) = (6.0f32, 26.0f32);
    let (lo, hi) = (margin, SIZE as f32 - margin);
    let mut inside = 0;
    for sy in 0..4 {
        for sx in 0..4 {
            let px = x as f32 + (sx as f32 + 0.5) / 4.0;
            let py = y as f32 + (sy as f32 + 0.5) / 4.0;
            let cx = px.clamp(lo + radius, hi - radius);
            let cy = py.clamp(lo + radius, hi - radius);
            let in_box = px >= lo && px <= hi && py >= lo && py <= hi;
            let (ddx, ddy) = (px - cx, py - cy);
            if in_box && ddx * ddx + ddy * ddy <= radius * radius {
                inside += 1;
            }
        }
    }
    inside as f32 / 16.0
}

/// Draws letter icons with one font lookup for a batch.
pub struct Painter {
    accent: u32,
    font: Option<Option<FontVec>>,
}

impl Painter {
    pub fn new(accent: u32) -> Self {
        Self { accent, font: None }
    }

    /// RGBA pixels of a letter icon.
    pub fn pixels(&mut self, label: &str) -> Vec<u8> {
        let font = self.font.get_or_insert_with(load_font);
        let usable = font
            .as_ref()
            .filter(|font| label.chars().all(|ch| font.glyph_id(ch).0 != 0));
        let coverage = label_coverage(label, usable);
        let accent = self.accent;
        let ink: u32 = if luminance(accent) > 0.55 {
            0x161618
        } else {
            0xf5f5f5
        };
        let mix = |shift: u32, c: f32| {
            let a = ((accent >> shift) & 0xff) as f32;
            let b = ((ink >> shift) & 0xff) as f32;
            (a + (b - a) * c).round() as u8
        };
        let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
        for y in 0..SIZE {
            for x in 0..SIZE {
                let c = coverage[(y * SIZE + x) as usize];
                pixels.extend_from_slice(&[
                    mix(16, c),
                    mix(8, c),
                    mix(0, c),
                    (shape_alpha(x, y) * 255.0).round() as u8,
                ]);
            }
        }
        pixels
    }

    pub fn write(&mut self, path: &Path, label: &str) -> std::io::Result<()> {
        let png = encode_png(SIZE, SIZE, &self.pixels(label));
        write_atomic(path, &png)
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, data)?;
    std::fs::rename(&temp, path)
}

// ---------------------------------------------------------------- PNG

fn crc32(chunks: &[&[u8]]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for data in chunks {
        for &byte in *data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    0xedb8_8320 ^ (crc >> 1)
                } else {
                    crc >> 1
                };
            }
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
}

/// RGBA8 PNG with stored (uncompressed) deflate blocks.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in rgba.chunks(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (index, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(index + 1 == blocks.len()));
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Width and height of a PNG whose signature, IHDR and chunk CRCs are valid.
pub fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 33 || &data[..8] != b"\x89PNG\r\n\x1a\n" || &data[12..16] != b"IHDR" {
        return None;
    }
    let mut pos = 8;
    let mut seen_end = false;
    while pos + 12 <= data.len() {
        let len = u32::from_be_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        let end = pos.checked_add(12)?.checked_add(len)?;
        if end > data.len() {
            return None;
        }
        let kind = &data[pos + 4..pos + 8];
        let body = &data[pos + 8..pos + 8 + len];
        let crc = u32::from_be_bytes(data[end - 4..end].try_into().ok()?);
        if crc != crc32(&[kind, body]) {
            return None;
        }
        if kind == b"IEND" {
            seen_end = true;
            break;
        }
        pos = end;
    }
    if !seen_end {
        return None;
    }
    let width = u32::from_be_bytes(data[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(data[20..24].try_into().ok()?);
    (width > 0 && height > 0 && width <= 4096 && height <= 4096).then_some((width, height))
}

// ---------------------------------------------------------------- fetch

pub const FETCH_LIMIT: usize = 512 * 1024;

/// The largest PNG image inside an ICO file (modern favicons embed PNG).
fn png_from_ico(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 6 || data[..4] != [0, 0, 1, 0] {
        return None;
    }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut best: Option<(u32, Vec<u8>)> = None;
    for index in 0..count {
        let entry = data.get(6 + index * 16..6 + (index + 1) * 16)?;
        let size = u32::from_le_bytes(entry[8..12].try_into().ok()?) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().ok()?) as usize;
        let image = data.get(offset..offset.checked_add(size)?)?;
        if let Some((w, h)) = png_size(image) {
            if best.as_ref().is_none_or(|(area, _)| w * h > *area) {
                best = Some((w * h, image.to_vec()));
            }
        }
    }
    best.map(|(_, png)| png)
}

/// A validated PNG from fetched bytes (PNG directly or inside an ICO).
pub fn validate_fetched(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() > FETCH_LIMIT {
        return None;
    }
    if png_size(data).is_some() {
        return Some(data.to_vec());
    }
    png_from_ico(data)
}

/// One HTTPS GET with a 5 s timeout and a 512 KB limit, through curl.
fn https_get(url: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-redirs",
            "3",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            "5",
            "--max-filesize",
            &FETCH_LIMIT.to_string(),
            "--",
            url,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("curl is needed for --fetch-icon: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    if output.stdout.len() > FETCH_LIMIT {
        return Err("icon larger than 512 KB".into());
    }
    Ok(output.stdout)
}

/// Fetch the site's apple-touch-icon, or its favicon if there is none.
/// Only ever called for an explicit `--fetch-icon`.
pub fn fetch_site_icon(host: &str) -> Result<Vec<u8>, String> {
    let mut errors = Vec::new();
    for name in ["apple-touch-icon.png", "favicon.ico"] {
        let url = format!("https://{host}/{name}");
        match https_get(&url) {
            Ok(data) => match validate_fetched(&data) {
                Some(png) => return Ok(png),
                None => errors.push(format!("{name}: not a usable PNG image")),
            },
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    Err(errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_icon_is_a_valid_128px_png() {
        let png = encode_png(SIZE, SIZE, &Painter::new(0x8aa87e).pixels("T"));
        assert_eq!(png_size(&png), Some((128, 128)));
        let mut corrupt = png.clone();
        corrupt[40] ^= 0xff;
        assert_eq!(png_size(&corrupt), None);
    }

    #[test]
    fn letter_icon_has_shape_and_ink() {
        let pixels = Painter::new(0x5a5a60).pixels("M");
        let at = |x: usize, y: usize| &pixels[(y * SIZE as usize + x) * 4..][..4];
        assert_eq!(at(0, 0)[3], 0, "corner is transparent");
        assert_eq!(at(20, 64), [0x5a, 0x5a, 0x60, 255], "edge is accent");
        let inked = pixels.chunks(4).filter(|p| p[0] > 0xc0).count();
        assert!(inked > 300, "the letter is drawn ({inked} px)");
    }

    #[test]
    fn letters_come_from_the_name() {
        assert_eq!(letter_for("teams"), 'T');
        assert_eq!(letter_for("  öppna"), 'Ö');
        assert_eq!(letter_for("365 Admin"), '3');
        assert_eq!(letter_for("--"), '?');
        assert_eq!(label_for("Teams", ["Outlook", "Gemini"]), "T");
        assert_eq!(label_for("Claude", ["ChatGPT", "Teams"]), "Cl");
        assert_eq!(label_for("ChatGPT", ["Claude"]), "Ch");
        assert_eq!(label_for("X", ["Xbox"]), "X");
    }

    #[test]
    fn two_letter_labels_render() {
        let png = encode_png(SIZE, SIZE, &Painter::new(0xc96050).pixels("Ch"));
        assert_eq!(png_size(&png), Some((128, 128)));
    }

    #[test]
    fn accent_follows_the_theme() {
        let dir = std::env::temp_dir().join(format!("nwa-accent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("niruwm/themes")).unwrap();
        assert_eq!(accent(&dir), 0x5a5a60);
        std::fs::write(
            dir.join("niruwm/config.toml"),
            "theme = \"niru-nord\" # x\n",
        )
        .unwrap();
        assert_eq!(accent(&dir), 0x88c0d0);
        std::fs::write(
            dir.join("niruwm/appearance.toml"),
            "[colors]\naccent = \"112233\"\n",
        )
        .unwrap();
        assert_eq!(accent(&dir), 0x112233);
        std::fs::write(
            dir.join("niruwm/themes/niru-nord.toml"),
            "focus = \"445566\"\n",
        )
        .unwrap();
        assert_eq!(accent(&dir), 0x445566);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fetched_icons_are_validated() {
        let png = encode_png(2, 2, &[255; 16]);
        assert!(validate_fetched(&png).is_some());
        assert!(validate_fetched(b"<html>not an icon</html>").is_none());
        let mut ico = vec![0, 0, 1, 0, 1, 0];
        ico.extend_from_slice(&[2, 2, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&png);
        assert_eq!(validate_fetched(&ico), Some(png));
        assert!(validate_fetched(&vec![0u8; FETCH_LIMIT + 1]).is_none());
    }
}

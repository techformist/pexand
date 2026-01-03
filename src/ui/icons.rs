//! Icon loading and parsing utilities

use iced::window;

/// Load the application icon from embedded bytes or filesystem
pub fn load_icon() -> Option<window::Icon> {
    // Try to load embedded icon bytes first
    const ICON_BYTES: &[u8] = include_bytes!("../../assets/icon.ico");

    if let Some((width, height, rgba)) = parse_ico_to_rgba(ICON_BYTES) {
        if let Ok(icon) = window::icon::from_rgba(rgba, width, height) {
            return Some(icon);
        }
    }

    // Fallback: try to load from file system
    let possible_paths = vec![
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("assets/icon.ico"))),
        Some(std::path::PathBuf::from("assets/icon.ico")),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("icon.ico"))),
        Some(std::path::PathBuf::from("icon.ico")),
    ];

    for path in possible_paths.into_iter().flatten() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Some((width, height, rgba)) = parse_ico_to_rgba(&bytes) {
                if let Ok(icon) = window::icon::from_rgba(rgba, width, height) {
                    return Some(icon);
                }
            }
        }
    }

    None
}

/// Simple ICO parser to extract RGBA data
fn parse_ico_to_rgba(ico_data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    // ICO format: header (6 bytes) + directory entries (16 bytes each)
    if ico_data.len() < 6 {
        return None;
    }

    let count = u16::from_le_bytes([ico_data[4], ico_data[5]]) as usize;
    if count == 0 || ico_data.len() < 6 + count * 16 {
        return None;
    }

    // Find 32x32 or largest entry
    let mut best_entry = None;
    let mut best_size = 0u32;

    for i in 0..count {
        let offset = 6 + i * 16;
        let width = ico_data[offset] as u32;
        let width = if width == 0 { 256 } else { width };
        let height = ico_data[offset + 1] as u32;
        let height = if height == 0 { 256 } else { height };

        let size = width * height;
        if width == 32 && height == 32 {
            best_entry = Some(offset);
            break;
        } else if size > best_size {
            best_size = size;
            best_entry = Some(offset);
        }
    }

    let entry_offset = best_entry?;
    let width = ico_data[entry_offset] as u32;
    let _width = if width == 0 { 256 } else { width };
    let height = ico_data[entry_offset + 1] as u32;
    let _height = if height == 0 { 256 } else { height };

    let data_size = u32::from_le_bytes([
        ico_data[entry_offset + 8],
        ico_data[entry_offset + 9],
        ico_data[entry_offset + 10],
        ico_data[entry_offset + 11],
    ]);

    let data_offset = u32::from_le_bytes([
        ico_data[entry_offset + 12],
        ico_data[entry_offset + 13],
        ico_data[entry_offset + 14],
        ico_data[entry_offset + 15],
    ]) as usize;

    if data_offset + data_size as usize > ico_data.len() {
        return None;
    }

    let image_data = &ico_data[data_offset..data_offset + data_size as usize];

    // Check if it's a PNG (starts with PNG signature)
    if image_data.len() > 8 && &image_data[1..4] == b"PNG" {
        // For now, we can't parse PNG without adding dependencies
        return None;
    }

    // Try to parse as BMP (most common in ICO)
    // BMP in ICO doesn't have the 14-byte file header
    if image_data.len() < 40 {
        return None;
    }

    let bmp_width =
        i32::from_le_bytes([image_data[4], image_data[5], image_data[6], image_data[7]]) as u32;

    let bmp_height =
        i32::from_le_bytes([image_data[8], image_data[9], image_data[10], image_data[11]]).abs()
            as u32
            / 2; // ICO BMPs have double height (image + mask)

    let bits_per_pixel = u16::from_le_bytes([image_data[14], image_data[15]]);

    if bits_per_pixel != 32 {
        // Only support 32-bit RGBA for simplicity
        return None;
    }

    let header_size = 40;
    let pixel_data_offset = header_size;
    let pixel_data_size = (bmp_width * bmp_height * 4) as usize;

    if pixel_data_offset + pixel_data_size > image_data.len() {
        return None;
    }

    // Extract BGRA pixel data and convert to RGBA
    let mut rgba = Vec::with_capacity(pixel_data_size);
    let pixels = &image_data[pixel_data_offset..pixel_data_offset + pixel_data_size];

    // BMP is stored bottom-up, so we need to flip it
    for y in (0..bmp_height).rev() {
        for x in 0..bmp_width {
            let offset = ((y * bmp_width + x) * 4) as usize;
            if offset + 3 < pixels.len() {
                let b = pixels[offset];
                let g = pixels[offset + 1];
                let r = pixels[offset + 2];
                let a = pixels[offset + 3];
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(a);
            }
        }
    }

    Some((bmp_width, bmp_height, rgba))
}

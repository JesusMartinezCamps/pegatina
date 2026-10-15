use anyhow::{bail, Context, Result};
use image::{ImageFormat, RgbaImage};
use std::io::Cursor;

pub const CF_DIB: u32 = 8;
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_HDROP: u32 = 15;
pub const CF_DIBV5: u32 = 17;

const DROPFILES_SIZE: usize = 20;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
const BI_ALPHABITFIELDS: u32 = 6;

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

pub fn utf16_nul(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect()
}

pub fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = utf16_units(bytes)
        .into_iter()
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

pub fn parse_dropfiles(data: &[u8]) -> Vec<String> {
    if data.len() < DROPFILES_SIZE {
        return Vec::new();
    }
    let offset = u32_at(data, 0) as usize;
    let wide = u32_at(data, 16) != 0;
    let Some(body) = data.get(offset..) else {
        return Vec::new();
    };
    if wide {
        utf16_units(body)
            .split(|&u| u == 0)
            .take_while(|s| !s.is_empty())
            .map(String::from_utf16_lossy)
            .collect()
    } else {
        body.split(|&b| b == 0)
            .take_while(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }
}

pub fn build_dropfiles(paths: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend((DROPFILES_SIZE as u32).to_le_bytes());
    out.extend([0u8; 8]);
    out.extend(0u32.to_le_bytes());
    out.extend(1u32.to_le_bytes());
    for path in paths {
        out.extend(path.encode_utf16().flat_map(u16::to_le_bytes));
        out.extend([0u8; 2]);
    }
    out.extend([0u8; 2]);
    out
}

pub fn png_size(png: &[u8]) -> Result<(u32, u32)> {
    let img = image::load_from_memory_with_format(png, ImageFormat::Png)
        .context("El PNG del portapapeles no se puede leer")?;
    Ok((img.width(), img.height()))
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;
    Ok(out)
}

pub fn dib_to_png(dib: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    if dib.len() < 40 {
        bail!("La imagen del portapapeles está incompleta");
    }
    let header = u32_at(dib, 0) as usize;
    let bit_count = u16_at(dib, 14);
    let compression = u32_at(dib, 16);
    let colors_used = u32_at(dib, 32) as usize;
    let masks = match (header, compression) {
        (40, BI_BITFIELDS) => 12,
        (40, BI_ALPHABITFIELDS) => 16,
        _ => 0,
    };
    let palette_entries = if bit_count <= 8 && colors_used == 0 {
        1usize << bit_count
    } else {
        colors_used
    };
    let palette = palette_entries * 4;
    let offset = 14 + header + masks + palette;

    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend(b"BM");
    bmp.extend(((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend([0u8; 4]);
    bmp.extend((offset as u32).to_le_bytes());
    bmp.extend(dib);

    let mut rgba = image::load_from_memory_with_format(&bmp, ImageFormat::Bmp)
        .context("No se pudo leer la imagen del portapapeles")?
        .into_rgba8();
    // Un DIB de 32 bits sin máscaras no define alfa: muchos programas lo dejan a 0.
    let undefined_alpha = header == 40 && compression == BI_RGB && bit_count == 32;
    if undefined_alpha || rgba.pixels().all(|p| p.0[3] == 0) {
        rgba.pixels_mut().for_each(|p| p.0[3] = 255);
    }
    let (w, h) = rgba.dimensions();
    Ok((encode_png(&rgba)?, w, h))
}

pub fn png_to_dib(png: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory_with_format(png, ImageFormat::Png)
        .context("No se pudo leer la imagen")?
        .into_rgba8();
    let (w, h) = img.dimensions();
    let mut out = Vec::with_capacity(40 + (w * h * 4) as usize);
    out.extend(40u32.to_le_bytes());
    out.extend((w as i32).to_le_bytes());
    out.extend((h as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(BI_RGB.to_le_bytes());
    out.extend((w * h * 4).to_le_bytes());
    out.extend([0u8; 16]);
    for y in (0..h).rev() {
        for x in 0..w {
            let [r, g, b, a] = img.get_pixel(x, y).0;
            let over_white = |c: u8| ((c as u32 * a as u32 + 255 * (255 - a as u32)) / 255) as u8;
            out.extend([over_white(b), over_white(g), over_white(r), 255]);
        }
    }
    Ok(out)
}

pub fn app_name_from_exe(path: &str) -> Option<String> {
    let file = path.rsplit(['\\', '/']).next()?;
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    if stem.is_empty() {
        return None;
    }
    let known = match stem.to_ascii_lowercase().as_str() {
        "explorer" => Some("Explorador"),
        "notepad" => Some("Bloc de notas"),
        "msedge" => Some("Edge"),
        "winword" => Some("Word"),
        "excel" => Some("Excel"),
        "powerpnt" => Some("PowerPoint"),
        _ => None,
    };
    if let Some(name) = known {
        return Some(name.to_string());
    }
    let mut chars = stem.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_png() -> Vec<u8> {
        let mut img = RgbaImage::new(3, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(2, 0, image::Rgba([0, 255, 0, 255]));
        img.put_pixel(1, 1, image::Rgba([0, 0, 255, 255]));
        encode_png(&img).unwrap()
    }

    #[test]
    fn utf16_round_trip_stops_at_nul() {
        let bytes = utf16_nul("hola ñ €");
        assert_eq!(utf16_text(&bytes), "hola ñ €");
        let mut with_garbage = bytes.clone();
        with_garbage.extend([b'x', 0, b'y', 0]);
        assert_eq!(utf16_text(&with_garbage), "hola ñ €");
    }

    #[test]
    fn dropfiles_round_trip() {
        let paths = vec![
            r"C:\Users\usuario\a.txt".to_string(),
            r"D:\fotos\b ñ.png".to_string(),
        ];
        assert_eq!(parse_dropfiles(&build_dropfiles(&paths)), paths);
        assert!(parse_dropfiles(&[0u8; 4]).is_empty());
        assert!(parse_dropfiles(&build_dropfiles(&[])).is_empty());
    }

    #[test]
    fn dropfiles_ansi_variant() {
        let mut data = Vec::new();
        data.extend(20u32.to_le_bytes());
        data.extend([0u8; 16]);
        data.extend(b"C:\\a.txt\0C:\\b.txt\0\0");
        assert_eq!(parse_dropfiles(&data), vec!["C:\\a.txt", "C:\\b.txt"]);
    }

    #[test]
    fn png_dib_round_trip_keeps_pixels() {
        let png = sample_png();
        let dib = png_to_dib(&png).unwrap();
        let (back, w, h) = dib_to_png(&dib).unwrap();
        assert_eq!((w, h), (3, 2));
        let img = image::load_from_memory_with_format(&back, ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(img.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(2, 0).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(1, 1).0, [0, 0, 255, 255]);
        assert_eq!(png_size(&back).unwrap(), (3, 2));
    }

    #[test]
    fn dib_with_bitfields_header_is_read() {
        let mut dib = Vec::new();
        dib.extend(40u32.to_le_bytes());
        dib.extend(2i32.to_le_bytes());
        dib.extend(1i32.to_le_bytes());
        dib.extend(1u16.to_le_bytes());
        dib.extend(32u16.to_le_bytes());
        dib.extend(BI_BITFIELDS.to_le_bytes());
        dib.extend(8u32.to_le_bytes());
        dib.extend([0u8; 16]);
        dib.extend(0x00FF_0000u32.to_le_bytes());
        dib.extend(0x0000_FF00u32.to_le_bytes());
        dib.extend(0x0000_00FFu32.to_le_bytes());
        dib.extend([0, 0, 255, 0, 0, 255, 0, 0]);
        let (png, w, h) = dib_to_png(&dib).unwrap();
        assert_eq!((w, h), (2, 1));
        let img = image::load_from_memory_with_format(&png, ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(img.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(img.get_pixel(1, 0).0, [0, 255, 0, 255]);
    }

    fn v5_dib(alpha_mask: u32, bgra: [u8; 4]) -> Vec<u8> {
        let mut dib = Vec::new();
        dib.extend(124u32.to_le_bytes());
        dib.extend(1i32.to_le_bytes());
        dib.extend(1i32.to_le_bytes());
        dib.extend(1u16.to_le_bytes());
        dib.extend(32u16.to_le_bytes());
        dib.extend(BI_BITFIELDS.to_le_bytes());
        dib.extend(4u32.to_le_bytes());
        dib.extend([0u8; 16]);
        for mask in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF, alpha_mask] {
            dib.extend(mask.to_le_bytes());
        }
        dib.extend(0x7352_4742u32.to_le_bytes());
        dib.extend([0u8; 48]);
        dib.extend(4u32.to_le_bytes());
        dib.extend([0u8; 12]);
        dib.extend(bgra);
        dib
    }

    fn first_pixel(png: &[u8]) -> [u8; 4] {
        image::load_from_memory_with_format(png, ImageFormat::Png)
            .unwrap()
            .into_rgba8()
            .get_pixel(0, 0)
            .0
    }

    #[test]
    fn v5_dib_keeps_a_real_alpha_channel() {
        let (png, w, h) = dib_to_png(&v5_dib(0xFF00_0000, [0, 0, 255, 128])).unwrap();
        assert_eq!((w, h), (1, 1));
        assert_eq!(first_pixel(&png), [255, 0, 0, 128]);
    }

    #[test]
    fn v5_dib_with_all_zero_alpha_is_not_invisible() {
        let (png, _, _) = dib_to_png(&v5_dib(0xFF00_0000, [0, 0, 255, 0])).unwrap();
        assert_eq!(first_pixel(&png), [255, 0, 0, 255]);
        let (png, _, _) = dib_to_png(&v5_dib(0, [0, 0, 255, 0])).unwrap();
        assert_eq!(first_pixel(&png), [255, 0, 0, 255]);
    }

    #[test]
    fn truncated_dib_is_an_error() {
        assert!(dib_to_png(&[0u8; 10]).is_err());
    }

    #[test]
    fn app_names_are_friendly() {
        assert_eq!(
            app_name_from_exe(r"C:\Windows\explorer.exe").as_deref(),
            Some("Explorador")
        );
        assert_eq!(
            app_name_from_exe(r"C:\x\brave.exe").as_deref(),
            Some("Brave")
        );
        assert_eq!(
            app_name_from_exe(r"C:\x\WindowsTerminal.exe").as_deref(),
            Some("WindowsTerminal")
        );
        assert_eq!(app_name_from_exe("EXCEL.EXE").as_deref(), Some("Excel"));
        assert_eq!(app_name_from_exe(""), None);
    }
}

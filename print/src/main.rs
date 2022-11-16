use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use png::{BitDepth, ColorType};

const PIXELS_PER_BYTE: usize = 8;

/// Number of bytes per printer row
const PRINTER_BYTES_PER_ROW: usize = 48;

/// Horizontal pixels per row for the printer
const PRINTER_HORIZ_RES: usize = PRINTER_BYTES_PER_ROW * PIXELS_PER_BYTE;

pub const BITMAP_D24: &[u8] = b"\x1b\x2a\x21"; // 32: 24 dots double density,203dpi
pub const LS_SET: &[u8] = b"\x1b\x33";

fn main() -> Result<()> {
    let args = std::env::args().skip(1);

    let mut ctx = libusb::Context::new()?;

    let printer = pos58_usb::POS58USB::new(&mut ctx, Duration::from_secs(2))?;
    let mut writer = BufWriter::new(printer);

    //let mut printer = vec![];
    //let mut writer = BufWriter::new(&mut printer);

    for path in args {
        println!("Press enter when ready to print {}", path);
        let _ = std::io::stdin().read_line(&mut String::new());

        let image = load_bitmap_png(&path).context(path.clone())?;
        let image = bits_to_bools(&image);
        print_bitmap(&mut writer, &image)?;
    }

    drop(writer);
    //std::io::stdout().write_all(&printer)?;
    //std::io::stdout().flush()?;

    Ok(())
}

fn load_bitmap_png(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let decoder = png::Decoder::new(File::open(path)?);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();

    assert_eq!(info.bit_depth, BitDepth::One);
    assert_eq!(info.color_type, ColorType::Grayscale);
    assert_eq!(info.width as usize, PRINTER_HORIZ_RES);

    buf.truncate(info.buffer_size());

    Ok(buf)
}

fn bits_to_bools(image: &[u8]) -> Vec<bool> {
    image
        .iter()
        .map(|b| (0..8).map(move |i| (b << i) & 0x80 != 0))
        .flatten()
        .collect()
}

fn print_bitmap<W: Write>(mut printer: W, bitmap: &[bool]) -> Result<()> {
    // Sanity check, determine height
    let width = PRINTER_HORIZ_RES;

    let total_pixels = bitmap.len();
    assert_eq!(total_pixels % width, 0);
    assert!(bitmap.len() > 0);
    //let height = total_pixels / width;

    printer.write_all(LS_SET)?;
    printer.write(&[0])?;

    let bytes_per_line = 3 * 8 * width;
    for window in bitmap.chunks(bytes_per_line) {
        printer.write_all(BITMAP_D24)?;
        printer.write_all(&u16::to_le_bytes(3 * width as u16))?;

        for x in 0..width {
            for set in 0..3 {
                let mut b = 0;
                for bit in 0..8 {
                    let row = set * 8 + bit;
                    let idx = row * width + x;
                    let w = window.get(idx).copied().unwrap_or(false);

                    b <<= 1;
                    if w {
                        b |= 1;
                    };
                }
                printer.write(&[b])?;
            }
        }

        printer.write(b"\n")?;
    }

    printer.flush()?;

    Ok(())
}

fn write_pbm(path: impl AsRef<Path>, width: usize, height: usize, data: &[u8]) -> Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    assert!(width % 8 == 0);
    writeln!(f, "P1")?;
    writeln!(f, "{} {}", width, height)?;
    for row in data.chunks_exact(width / 8) {
        for &k in row {
            for bit in 0..8 {
                if k & (0x80 << bit) != 0 {
                    write!(f, "0")?;
                } else {
                    write!(f, "1")?;
                }
            }
        }
        writeln!(f)?;
    }
    Ok(())
}

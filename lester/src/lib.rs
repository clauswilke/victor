//! Lester, [Victor]’s cousin, makes raster graphics.
//!
//! This is a reimplementation of [Poppler]’s `pdftocairo` utility program as a Rust library.
//! It loads PDF documents with Poppler
//! and renders (rasterizes) them to in-memory pixel buffers with [cairo].
//! It can then export to PNG.
//!
//! Lester is primarily intended to help test the visual rendering of PDF files generated by Victor.
//! Reimplementing `pdftocairo` enables skipping the overhead of cross-process communication
//! and image compression/decompression.
//! It will also enable extracting (and testing) PDF metadata at the same time as rendering.
//!
//!
//! ## Requirements
//!
//! * [Poppler], with its `glib` wrapper API.
//! * [cairo]
//! * [pkg-config], at build time
//!
//!
//! [Victor]: https://github.com/SimonSapin/victor
//! [Poppler]: https://poppler.freedesktop.org/
//! [cairo]: https://www.cairographics.org/
//! [pkg-config]: https://www.freedesktop.org/wiki/Software/pkg-config/
//!
//!
//! ## Example
//!
//! Converting a PDF file to a series of PNG files:
//!
//! ```rust
//! use std::fs::File;
//! use std::io::Read;
//!
//! # fn _foo() -> Result<(), lester::LesterError> {
//! let mut bytes = Vec::new();
//! File::open("foo.pdf")?.read_to_end(&mut bytes)?;
//! let doc = lester::PdfDocument::from_bytes(&bytes)?;
//!
//! for (index, page) in doc.pages().enumerate() {
//!     let filename = format!("foo_page{}.png", index + 1);
//!     page.render()?.write_to_png_file(filename)?
//! }
//! # Ok(())
//! # }
//! ```

mod cairo;
mod errors;
mod poppler;

pub use crate::cairo::*;
pub use crate::errors::*;
pub use crate::poppler::*;

// Not re-exported:
mod cairo_ffi;
mod convert;
mod poppler_ffi;

/// `assert_eq!` for `Argb32Pixels::buffer`.
#[macro_export]
macro_rules! assert_pixels_eq {
    ($a: expr, $b: expr) => {{
        let a = $a;
        let b = $b;
        if a != b {
            panic!(
                "{} != {}\n[{}]\n[{}]",
                stringify!($a),
                stringify!($b),
                $crate::pixels_to_hex(a),
                $crate::pixels_to_hex(b)
            )
        }
    }};
}

#[doc(hidden)]
pub fn pixels_to_hex(pixels: &[u32]) -> String {
    pixels
        .iter()
        .map(|p| format!("{:08X}", p))
        .collect::<Vec<_>>()
        .join(", ")
}

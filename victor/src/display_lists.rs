use errors::VictorError;
use std::fs;
use std::io::{self, Write};
use std::path;

pub struct Document {
    pub pages: Vec<Page>,
}

impl Document {
    /// Encode this document to PDF and write it into the file with the given name.
    pub fn write_to_pdf_file<P: AsRef<path::Path>>(&self, filename: P) -> Result<(), VictorError> {
        self.write_to_pdf(io::BufWriter::new(fs::File::create(filename)?))
    }

    /// Encode this document to PDF and return a vector of bytes
    pub fn write_to_pdf_bytes(&self) -> Result<Vec<u8>, VictorError> {
        let mut bytes = Vec::new();
        self.write_to_pdf(&mut bytes)?;
        Ok(bytes)
    }

    /// Encode this document to PDF and write it to the given stream.
    ///
    /// Note: this may do many write calls.
    /// If a stream is backed by costly system calls (such as `File` or `TcpStream`),
    /// this method will likely perform better with that stream wrapped in `BufWriter`.
    ///
    /// See also the `write_to_png_file` method.
    pub fn write_to_pdf<W: Write>(&self, _stream: W) -> Result<(), VictorError> {
        Ok(())
    }
}

pub struct Page {
    pub width_in_ps_points: f32,
    pub height_in_ps_points: f32,
}

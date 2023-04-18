// catconf
// Copyright (C) 2023 Andrew Rioux
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! # Catconf
//!
//! For when you want:
//! 1. Runtime configuration for after the binary is compiled
//! 2. A single file binary
//!
//! This library allows for taking the final result binary, and just concatenating the configuration to the end:
//!
//! `cat target/debug/binary <(echo -n "CATCONF") conf > confedbinary`
//!
//! Great, but how to get the configuration out and use it in the code? catconf!
//!
//! It's use is pretty simple:
//!
//! ```
//! use catconf::ConfReaderOptions;
//!
//! # fn main () -> std::io::Result<()> {
//! let conf_reader = ConfReaderOptions::new(b"CATCONF".to_vec()).read_from_exe()?;
//! # Ok(())
//! # }
//! ```
//!
//! This returns a <code>[Vec]\<u8></code> which can be transformed further, by converting to UTF-8 and
//! combined with Serde, decompressing with zlib, etc

use std::{
    env,
    fs::OpenOptions,
    io::{self, prelude::*, SeekFrom},
};

/// Internal function used to just reference the current executable
pub(crate) fn open_current_exe() -> io::Result<std::fs::File> {
    OpenOptions::new().read(true).open(env::current_exe()?)
}

/// Builder struct to allow for configuring the eventual call to read from a file
/// It has two primary properties:
///
/// 1. Magic bytes: the bytes used to
/// 2. Window size: the size of the window used to scan the file. This library
///     will read in twice the window size to fill its internal buffer
///
/// # Example
///
/// ```
/// use catconf::ConfReaderOptions;
///
/// # fn main() -> std::io::Result<()> {
/// let conf = ConfReaderOptions::new(b"CATCONF".to_vec()).read_from_exe()?;
/// # Ok(())
/// # }
/// ```
pub struct ConfReaderOptions {
    magic_bytes_opt: Vec<u8>,
    window_size_opt: i64,
}

impl ConfReaderOptions {
    /// Create a new ConfReaderOptions builder with the magic bytes specified.
    ///
    /// The window size is initially set to 2048
    ///
    /// # Example
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut options = ConfReaderOptions::new(b"CATCONF".to_vec());
    /// let conf = options.window_size(4096).read_from_exe()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(bytes: Vec<u8>) -> Self {
        ConfReaderOptions {
            magic_bytes_opt: bytes,
            window_size_opt: 2048,
        }
    }

    /// Set the magic bytes to a different value
    ///
    /// # Example
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let options = ConfReaderOptions::new(b"CATCONF".to_vec())
    ///     .magic_bytes(b"NOTCATCONF".to_vec())
    ///     .read_from_exe()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn magic_bytes(&mut self, bytes: Vec<u8>) -> &mut Self {
        self.magic_bytes_opt = bytes;
        self
    }

    /// Sets the window size, influencing the amount of reads that are performed on disk
    ///
    /// # Example
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let conf = ConfReaderOptions::new(b"CATCONF".to_vec())
    ///     .window_size(4096)
    ///     .read_from_exe()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn window_size(&mut self, size: u32) -> &mut Self {
        self.window_size_opt = size as i64;
        self
    }

    /// Takes the configuration options provided and actually reads from the input file to
    /// gather the configuration
    ///
    /// # Example
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// # let buff = vec![0;4096];
    /// # let mut input = std::io::Cursor::new(&buff);
    /// let conf = ConfReaderOptions::new(b"CATCONF".to_vec())
    ///     .read(&mut input);
    /// # Ok(())
    /// # }
    /// ```
    pub fn read<F>(&self, input: &mut F) -> io::Result<Vec<u8>>
    where
        F: Seek + Read,
    {
        read_from_file(&self.magic_bytes_opt, self.window_size_opt, input)
    }

    /// Helper method to go along with [`ConfReaderOptions::read`] in order to read from the
    /// program currently checking for configuration
    ///
    /// Functionally equivalent to:
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut current_exe = std::fs::OpenOptions::new().read(true).open(std::env::current_exe()?)?;
    /// let conf = ConfReaderOptions::new(b"CATCONF".to_vec()).read(&mut current_exe)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use catconf::ConfReaderOptions;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let conf = ConfReaderOptions::new(b"CATCONF".to_vec()).read_from_exe()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_from_exe(&self) -> io::Result<Vec<u8>> {
        let mut cur_exe = open_current_exe()?;
        self.read(&mut cur_exe)
    }
}

/// Useful if you just want to read from the current exe without bothering to use the builder
///
/// # Example
///
/// ```
/// use catconf::read_from_exe;
///
/// # fn main() -> std::io::Result<()> {
/// let conf = read_from_exe(b"CATCONF", 4096)?;
/// # Ok(())
/// # }
/// ```
pub fn read_from_exe(magic_bytes: &[u8], window_size: i64) -> io::Result<Vec<u8>> {
    let mut cur_exe = open_current_exe()?;
    read_from_file(magic_bytes, window_size, &mut cur_exe)
}

/// Allows for reading for configuration from the end of a file by looking for magic bytes
///
/// # Example
///
/// ```no_run
/// use catconf::read_from_file;
///
/// # fn main() -> std::io::Result<()> {
/// # let buff = vec![0; 4096];
/// # let mut input = std::io::Cursor::new(&buff);
/// let conf = read_from_file(b"CATCONF", 2048, &mut input)?;
/// # Ok(())
/// # }
/// ```
pub fn read_from_file<F>(magic_bytes: &[u8], window_size: i64, input: &mut F) -> io::Result<Vec<u8>>
where
    F: Seek + Read,
{
    let buffer_size = window_size * 2;
    let mut current_window_index: i64 = 1;
    let mut current_read_buffer = vec![0u8; buffer_size as usize];

    loop {
        input.seek(SeekFrom::End(-((current_window_index + 1) * window_size)))?;
        let bytes_read = input.read(&mut current_read_buffer[..])?;

        if bytes_read < window_size as usize {
            break Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "reached beginning of the file without finding magic bytes",
            ));
        }

        if let Some(pos) = current_read_buffer
            .windows(magic_bytes.len())
            .position(|window| window == magic_bytes)
        {
            let conf_buffer_size = window_size - pos as i64 - magic_bytes.len() as i64
                + (current_window_index * window_size);
            let mut conf_buffer = vec![0; conf_buffer_size as usize];

            input.seek(SeekFrom::End(-conf_buffer_size))?;
            input.read(&mut conf_buffer[..])?;

            break Ok(conf_buffer);
        }

        current_window_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// Simplest use case
    #[test]
    fn pulls_basic_data() {
        let input_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];
        let header = [1, 2, 3, 4];
        let data = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

        let mut buf = Cursor::new(&input_data);

        assert_eq!(&read_from_file(&header, 16, &mut buf).unwrap(), &data);
    }

    /// Check to make sure reads occur when going across window boundaries. For instance, in
    /// this test the boundary will be split such that the first read reads "2,3,4,1,1,1...", missing
    /// the boundary
    #[test]
    fn pulls_data_over_boundary() {
        let input_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];
        let header = [1, 2, 3, 4];
        let data = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

        let mut buf = Cursor::new(&input_data);

        assert_eq!(&read_from_file(&header, 15, &mut buf).unwrap(), &data);
    }
}

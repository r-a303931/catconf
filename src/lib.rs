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

use std::{
    env,
    fs::OpenOptions,
    io::{self, prelude::*, SeekFrom},
};

pub fn open_current_exe() -> io::Result<std::fs::File> {
    OpenOptions::new().read(true).open(env::current_exe()?)
}

pub struct ConfReaderOptions {
    magic_bytes_opt: Vec<u8>,
    window_size_opt: i64,
    buffer_size_opt: Option<i64>,
}

impl ConfReaderOptions {
    pub fn new(bytes: Vec<u8>) -> Self {
        ConfReaderOptions {
            magic_bytes_opt: bytes,
            window_size_opt: 2048,
            buffer_size_opt: None,
        }
    }

    pub fn magic_bytes(&mut self, bytes: Vec<u8>) -> &mut Self {
        self.magic_bytes_opt = bytes;
        self
    }

    pub fn window_size(&mut self, size: u32) -> &mut Self {
        self.window_size_opt = size as i64;
        self
    }

    pub fn buffer_size(&mut self, size: u32) -> &mut Self {
        self.buffer_size_opt = Some(size as i64);
        self
    }

    pub fn read<F>(&mut self, mut input: F) -> io::Result<Vec<u8>>
    where
        F: Seek + Read,
    {
        let magic_bytes = &self.magic_bytes_opt;
        let window_size = self.window_size_opt;
        let buffer_size = self.buffer_size_opt.unwrap_or_else(|| window_size * 2);

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

    pub fn read_from_exe(&mut self) -> io::Result<Vec<u8>> {
        self.read(open_current_exe()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}

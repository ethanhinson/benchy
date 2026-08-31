use std::fs::File;
use std::io::{Read, Seek, SeekFrom, stdin};
use std::path::Path;

use anyhow::{Context, Result};

use crate::units::parse_byte_count;

pub fn read_input(
    path: Option<&Path>,
    skip: Option<&str>,
    length: Option<&str>,
    block_size: u64,
) -> Result<Vec<u8>> {
    let mut data = if let Some(path) = path {
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        let mut buf = Vec::new();
        stdin().lock().read_to_end(&mut buf)?;
        buf
    };

    let file_len = data.len() as u64;

    let skip_bytes = match skip {
        None => 0,
        Some(s) => {
            let signed = parse_signed_count(s, block_size)?;
            if signed < 0 {
                (file_len as i64 + signed).max(0) as u64
            } else {
                signed as u64
            }
        }
    };

    if skip_bytes as usize >= data.len() {
        data.clear();
    } else {
        data.drain(0..skip_bytes as usize);
    }

    if let Some(len_str) = length {
        let len = parse_byte_count(len_str, block_size)? as usize;
        data.truncate(len);
    }

    Ok(data)
}

fn parse_signed_count(input: &str, block_size: u64) -> Result<i64> {
    let input = input.trim();
    if input.starts_with('-') {
        let positive = parse_byte_count(&input[1..], block_size)?;
        return Ok(-(positive as i64));
    }
    Ok(parse_byte_count(input, block_size)? as i64)
}

#[allow(dead_code)]
pub fn read_file_size(path: &Path) -> Result<u64> {
    let mut file = File::open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    Ok(size)
}

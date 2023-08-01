use pathtracer::math::*;

use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::num::ParseFloatError;
use std::path::{Path, PathBuf};

use native_dialog::FileDialog;

use regex::Regex;

use lazy_static::lazy_static;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    FormatError(String),
    IoError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FormatError(err) => f.write_fmt(format_args!("Invalid format: {}", err)),
            Self::IoError(err) => f.write_fmt(format_args!("IO error: {}", err)),
        }
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Error::FormatError(err.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err.to_string())
    }
}

#[allow(dead_code)]
pub fn parse_sample(s: &str) -> Option<i32> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"_(\d+)spp").unwrap();
    }

    let capture = RE.captures_iter(s).next()?;
    match capture[1].parse::<i32>() {
        Err(_) => None,
        Ok(val) => Some(val),
    }
}

pub fn add_suffix(path: &Path, suffix: &str) -> PathBuf {
    let new_name = format!("{}{}", path.file_stem().unwrap().to_str().unwrap(), suffix);

    let mut result = path.to_owned();
    result.set_file_name(new_name);
    if let Some(ext) = path.extension() {
        result.set_extension(ext);
    }

    result
}

pub fn get_open_file_name(description: &str, filters: &[&str]) -> Option<PathBuf> {
    FileDialog::new()
        .add_filter(description, filters)
        .show_open_single_file()
        .unwrap()
}

pub fn get_save_file_name(description: &str, filters: &[&str]) -> Option<PathBuf> {
    FileDialog::new()
        .add_filter(description, filters)
        .show_save_single_file()
        .unwrap()
}

pub fn save_pfm(path: &Path, width: u32, height: u32, data: &[Vector3]) -> Result<(), Error> {
    let mut file = File::create(path)?;

    // "PF" = 3 channels
    file.write_all("PF\n".as_bytes())?;
    file.write_all(format!("{} {}\n", width, height).as_bytes())?;
    file.write_all("-1.0\n".as_bytes())?;

    for y in 0..height {
        for x in 0..width {
            let color = data[(x + y * width) as usize];
            file.write_all(&color.x.to_le_bytes())?;
            file.write_all(&color.y.to_le_bytes())?;
            file.write_all(&color.z.to_le_bytes())?;
        }
    }

    Ok(())
}

pub fn save_denoise(
    output: &Path,
    path: &Path,
    albedo: &Path,
    normals: &Path,
) -> Result<(), Error> {
    let mut file = File::create(output)?;

    let mut result = output.to_owned();
    result.set_extension("pfm");

    let content = format!(
        "& ${{env:OIDN_BIN}} --hdr {} --alb {} --nrm {} -o {}\n",
        path.to_str().unwrap(),
        albedo.to_str().unwrap(),
        normals.to_str().unwrap(),
        result.to_str().unwrap()
    );

    file.write_all(content.as_bytes())?;

    Ok(())
}

#[allow(dead_code)]
fn read_float(reader: &mut impl BufRead) -> Result<f32, Error> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

#[allow(dead_code)]
pub fn load_pfm(path: &Path) -> Result<(i32, i32, Vec<Vector3>), Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut header = String::new();
    let _ = reader.read_line(&mut header)?;

    if header != *"PF\n" {
        return Err(Error::FormatError("Invalid header".to_owned()));
    }

    let mut width_height = String::new();
    let _ = reader.read_line(&mut width_height)?;

    let width_height: Vec<i32> = width_height
        .trim()
        .split(' ')
        .map(|t| t.parse::<i32>())
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .collect();

    if width_height.len() != 2 {
        return Err(Error::FormatError("Invalid width or height".to_owned()));
    }

    let (width, height) = (width_height[0], width_height[1]);

    let mut scale = String::new();
    let _ = reader.read_line(&mut scale)?;

    let scale = scale.trim().parse::<f32>()?;
    if scale != -1.0 {
        return Err(Error::FormatError(format!("Invalid scale: {}", scale)));
    }

    let mut image_data = Vec::with_capacity((width * height) as usize);

    for _ in 0..height {
        for _ in 0..width {
            image_data.push(Vector3::new(
                read_float(&mut reader)?,
                read_float(&mut reader)?,
                read_float(&mut reader)?,
            ));
        }
    }

    Ok((width, height, image_data))
}

#[cfg(test)]
mod tests {
    use super::{add_suffix, get_open_file_name, get_save_file_name, load_pfm, parse_sample};
    use std::path::Path;

    #[test]
    fn test_parse() {
        assert_eq!(parse_sample("output_12spp.pfm"), Some(12));
        assert_eq!(parse_sample("output_1spp.pfm"), Some(1));
        assert_eq!(parse_sample("test_01spp.pfm"), Some(1));
        assert_eq!(parse_sample("output_spp.pfm"), None);
        assert_eq!(parse_sample("output_15pps.pfm"), None);
    }

    #[test]
    fn test_open_file() {
        assert_eq!(get_open_file_name("pfm image", &["pfm"]), None);
    }

    #[test]
    fn test_save_file() {
        assert_eq!(get_save_file_name("pfm image", &["pfm"]), None);
    }

    #[test]
    fn test_add_suffix() {
        assert_eq!(
            add_suffix(Path::new("c:\\path\\file.txt"), "_test"),
            Path::new("c:\\path\\file_test.txt")
        );
        assert_eq!(
            add_suffix(Path::new("c:\\path\\test"), "_test"),
            Path::new("c:\\path\\test_test")
        );
    }

    #[test]
    fn test_read_pfm() {
        let _ = load_pfm(Path::new("./test_trace_4spp.pfm"));
    }
}

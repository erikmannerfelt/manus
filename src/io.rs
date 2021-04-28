use serde_json::Value as Json;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

/// Try to parse and return a filepath.
///
/// # Arguments
/// - `filepath_str`: A relative or absolute filepath string.
/// - `expected_extension`: Optional. The expected extension. An extension-less path will be assigned it.
///
/// # Errors
/// If the path does not exist or an incorrect path/extension was given.
///
/// # Returns
/// A filepath, if a file with its name exists and it has the correct extension.
pub fn parse_filepath(
    filepath_str: &str,
    expected_extension: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create a PathBuf from the input string.
    let mut path = PathBuf::from(filepath_str);

    // If expected_extension was given, make sure they match.
    if let Some(ext) = expected_extension {
        // If the path has an extension, validate it to the expected one.
        // If the path has no extension, append the expected_extension to the path.
        match path.extension() {
            Some(ext2) => {
                if ext2 != ext {
                    return Err(
                        format!("Incorrect extension: {:?}. Expected: {}", ext2, ext).into(),
                    );
                }
            }
            None => {
                path.set_extension(ext);
            }
        }
    }
    // Check that the file exists.
    if !path.is_file() {
        return Err("File not found".into());
    }
    Ok(path)
}

/// Parse an input path by either reading from disk or from stdin.
///
/// # Arguments
/// * `input_str`: An input string to be parsed as a filepath or "-" to read from stdin.
/// * `output_path`: Optional. A string to parse as an output path. If None, create a fitting path.
///
/// # Returns
/// The parsed lines as a vector of strings and a fitting path for the output.
pub fn get_lines_and_output_path(
    input_str: &str,
    output_path: Option<&str>,
) -> Result<(Vec<String>, PathBuf), Box<dyn std::error::Error>> {
    let filepath: PathBuf;
    let lines: Vec<String>;

    // If the path is "-", read tex from stdin
    if input_str.trim() == "-" {
        lines = match read_tex_from_stdin() {
            Ok(l) => l,
            Err(e) => return Err(e),
        };
        // Simply assign the filepath to something generic. If the output path is specified,
        // this is obsolete.
        filepath = PathBuf::from("main.tex");
    } else {
        // Check that the file exists and return a valid PathBuf.
        filepath = match parse_filepath(&input_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(e),
        };

        // Read and merge all tex files.
        lines = match crate::merge_tex(&filepath) {
            Ok(l) => l,
            Err(e) => return Err(e),
        };
    }

    // Either get the filepath from the OUTPUT argument, or call it the same filename as the
    // input but with a changed extension.
    let pdf_filepath = match output_path {
        Some(x) => PathBuf::from(x),
        None => {
            let mut fp = PathBuf::from(filepath.file_name().unwrap());
            fp.set_extension("pdf");
            fp
        }
    };

    Ok((lines, pdf_filepath))
}

/// Read a datafile either from stdin or from disk.
///
/// # Arguments
/// * `input_str`: An input string to be parsed as a filepath or "-" to read from stdin.
///
/// # Returns
/// The parsed data file.
pub fn get_data_from_str(input_str: &str) -> Result<Json, Box<dyn std::error::Error>> {
    match input_str.trim() == "-" {
        true => read_data_from_stdin(),
        false => read_data(&PathBuf::from(input_str)),
    }
}

/// Read a datafile from stdin.
fn read_data_from_stdin() -> Result<Json, Box<dyn std::error::Error>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;

    Ok(serde_json::from_str(&buf)?)
}

/// Read a tex file as a vector of Strings
///
/// # Arguments
/// - `filepath`: A relative or absolute filepath.
///
/// # Errors
/// Fails if the file was not found or
pub fn read_tex(filepath: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Check that the file exists.
    if !filepath.is_file() {
        return Err(format!("File not found: {}", filepath.to_str().unwrap()).into());
    };

    // Open the file.
    let file = File::open(&filepath)?;
    let mut reader = std::io::BufReader::new(file);

    // Read the contents of the file into a buffer.
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;

    // Split the content of the buffer into separate lines.
    let lines: Vec<String> = buffer.lines().map(|s| s.to_owned()).collect();

    Ok(lines)
}

/// Read tex data from stdin.
pub fn read_tex_from_stdin() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;

    let lines: Vec<String> = buf.lines().map(|s| s.to_owned()).collect();

    Ok(lines)
}

/// Read a json data file into an arbitrary JSON dictionary.
pub fn read_data(filepath: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let file = File::open(filepath)?;
    let mut reader = std::io::BufReader::new(file);

    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    let extension = filepath
        .extension()
        .expect("Data file read with no extension!")
        .to_str()
        .unwrap();

    let data: Json = match extension {
        "json" => serde_json::from_str(&buf)?,
        "toml" => toml::from_str(&buf)?,
        s => return Err(format!("Could not read data type: {}", s).into()),
    };
    Ok(data)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_filepath() {
        parse_filepath("tests/data/case1/main.tex", Some("tex")).expect("This should exist");

        parse_filepath("tests/data/case1/main.tex", Some("text")).expect_err("This should fail");

        parse_filepath("tests/data/case1/main", Some("tex")).expect("This should pass");

        parse_filepath("Cargo.toml", Some("toml")).expect("This should pass");
    }
}

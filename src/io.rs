use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
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

/// Read a json data file into an arbitrary JSON dictionary.
pub fn read_data(filepath: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let file = File::open(filepath)?;
    let reader = std::io::BufReader::new(file);

    let data: serde_json::Value = serde_json::from_reader(reader)?;

    Ok(data)
}

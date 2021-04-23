use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::File;
use clap::{App, Arg};
use std::io::{self,Write};
use handlebars::{self, handlebars_helper};


handlebars_helper!(upper: | s: str | s.to_uppercase());
handlebars_helper!(lower: |s:str| s.to_lowercase());
handlebars_helper!(round: |v: f64| v.round());
handlebars_helper!(round_1: |v: f64| format!("{:.1}", v));
handlebars_helper!(round_2: |v: f64| format!("{:.2}", v));
handlebars_helper!(round_3: |v: f64| format!("{:.3}", v));


fn main() -> io::Result<()> {


    match parse_cli_args() {
        Ok(x) => io::stdout().write_all(x.as_bytes())?,
        Err(x) => io::stderr().write_all(x.as_bytes())?
    };

    Ok(())
}


///Handle the CLI arguments.
///
/// # Returns
///
/// A string to write to stdout (Ok) or to stderr (Err).
fn parse_cli_args() -> Result<String, String> {
    // Create a new app
    let matches = App::new("manus")
        .version("0.1.0")
        .author("Erik Mannerfelt")
        .about("Handle tex manuscripts.")
        // Create the 'build' subcommand for building pdfs.
        .subcommand(App::new("build")
            .about("Render the manuscript")
            .arg(Arg::new("INPUT")
                .about("The input root tex file.")
                .required(true)
                .index(1)
            )
            .arg(Arg::new("OUTPUT")
                .about("The output pdf path. Defaults to the current directory.")
                .required(false)
                .index(2)
            )
            .arg(Arg::new("DATA")
                .about("Data file")
                .short('d')
                .long("data")
            )
        )
        .subcommand(App::new("convert")
            .about("Convert to different formats.")
            .arg(Arg::new("INPUT")
                .about("The input root tex file")
                .required(true)
                .index(1)
            )
            .arg(Arg::new("DATA")
                .about("Data file")
                .short('d')
                .long("data")
                .takes_value(true)
            )
            .arg(Arg::new("FORMAT")
                .about("Format. Choices: [tex]. Defaults to tex.")
                .short('f')
                .long("format")
            )
        )
        .subcommand(App::new("merge")
            .about("Merge 'input' clauses.")
            .arg(Arg::new("INPUT")
                .about("The input root tex file.")
                .required(true)
                .index(1)
            )
        )
        .get_matches();


    // 'build' subcommand parser.
    if let Some(ref matches) = matches.subcommand_matches("build") {
        
        // Parse the filepath.
        let path_str = matches.value_of("INPUT").expect("It's a required argument so this shouldn't fail.");

        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e))
        };

        // Read and merge all tex files.
        let lines = match merge_tex(&filepath) {
            Ok(l) => l,
            Err(e) => return Err(format!("{:?}", e))
        };
        // Render the PDF
        let pdf_data: Vec<u8> = tectonic::latex_to_pdf(lines.join("\n")).expect("oops");

        // Either get the filepath from the OUTPUT argument, or call it the same filename as the
        // input but with a changed extension.
        let pdf_filepath = match matches.value_of("OUTPUT") {
            Some(x) => PathBuf::from(x),
            None => {
                let mut fp = PathBuf::from(filepath.file_name().unwrap());
                fp.set_extension("pdf");
                fp
            }
        };
        // Create a new file and write the PDF data to it.
        let mut file = File::create(&pdf_filepath).expect("");
        file.write_all(&pdf_data).expect("");

    }

    // 'convert' subcommand parser
    if let Some(ref matches) = matches.subcommand_matches("convert") {

        // Parse the input.
        let path_str = matches.value_of("INPUT").expect("It's a reqired argument so this won't fail.");
        
        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e))
        };

        // Write the result to stdout if it worked or the error to stderr if it didn't.
        let mut lines = match merge_tex(&filepath) {
            Ok(lines) => lines,
            Err(message) => return Err(format!("{:?}", message))
        };

        // Parse the data argument and do template filling in case it was given.
        if let Some(data_path_str) = matches.value_of("DATA") {

            let data_path = match parse_filepath(&data_path_str, Some("json")) {
                Ok(fp) => fp,
                Err(e) => return Err(format!("{:?}", e))
            };

            let data = match read_data(&data_path) {
                Ok(d) => d,
                Err(e) => return Err(format!("{:?}", e))
            };

            lines = fill_data(&lines, &data);
        }

        // Return the text to write to stdout.
        return Ok(lines.join("\n"))

    }

    // 'merge' subcommand parser.
    if let Some(ref matches) = matches.subcommand_matches("merge") {

        // Parse the input path..
        let path_str = matches.value_of("INPUT").expect("It's a reqired argument so this won't fail.");
        
        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e))
        };

        // Write the result to stdout if it worked or the error to stderr if it didn't.
        match merge_tex(&filepath) {
            Ok(lines) => return Ok(lines.join("\n")),
            Err(message) => return Err(format!("{:?}", message))
        };
    }

    // If no return statements were reached. Write an empty string to stderr.
    Err("".into())

}

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
fn parse_filepath(filepath_str: &str, expected_extension: Option<&str>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    
    // Create a PathBuf from the input string.
    let mut path = PathBuf::from(filepath_str);

    // If expected_extension was given, make sure they match.
    if let Some(ext) = expected_extension {
        // If the path has an extension, validate it to the expected one.
        // If the path has no extension, append the expected_extension to the path.
        match path.extension() {
            Some(ext2) => {
                if ext2 != ext {
                    return Err(format!("Incorrect extension: {:?}. Expected: {}", ext2, ext).into());
                }},
            None => {path.set_extension(ext);}
        }

    }
    // Check that the file exists.
    if !path.is_file() {
        return Err("File not found".into());
    }
    Ok(path)
}


/// Read a tex file as a vector of Strings
///
/// # Arguments
/// - `filepath`: A relative or absolute filepath.
///
/// # Errors
/// Fails if the file was not found or 
fn read_tex(filepath: &PathBuf) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    // Check that the file exists.
    if !filepath.is_file() {
        return Err(format!("File not found: {}", filepath.to_str().unwrap()).into())
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


/// Read a tex file and recursively merge all \\input{} statements.
///
/// # Arguments
/// * `filepath`: A relative or absolute path to the main.tex.
fn merge_tex(filepath: &PathBuf) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    // Create the output line vector
    let mut lines: Vec<String> = Vec::new();

    // Parse the lines of the main file.
    let main_lines = read_tex(&filepath)?;

    // Loop over the lines and handle any \input clauses.
    for line in main_lines {
        // If it doesn't contain and input, just continue.
        if !line.contains(r"\input{") {
            lines.push(line);
            continue
        }
        let mut input_path = PathBuf::from(line.replace(r"\input{", "").replace("}", ""));

        if input_path.extension().is_none() {
            let _ = input_path.set_extension("tex");
        }

        if !input_path.is_file() {
            input_path = [filepath.parent().unwrap(), &input_path].iter().collect();
        }

        let input_lines = merge_tex(&PathBuf::from(&input_path))?;

        for input_line in input_lines {
            lines.push(input_line)
        }
    }
    Ok(lines)
}


/// Fill a vector of text with data using templating.
fn fill_data(lines: &Vec<String>, data: &serde_json::Value) -> Vec<String> {

    let mut new_lines: Vec<String> = Vec::new();

    let mut reg = handlebars::Handlebars::new();
    reg.register_helper("upper", Box::new(upper));
    reg.register_helper("lower", Box::new(lower));
    reg.register_helper("round", Box::new(round));
    reg.register_helper("round-1", Box::new(round_1));
    reg.register_helper("round-2", Box::new(round_2));
    reg.register_helper("round-3", Box::new(round_3));
    reg.set_strict_mode(true);

    for line in lines {
        match reg.render_template(line, data) {
            Ok(l) => new_lines.push(l),
            Err(e) => {
                let err = format!("WARNING: {}\n", e.as_render_error().unwrap().desc.replace(" in strict mode", ""));
                io::stderr().write_all(err.as_bytes()).unwrap();
                new_lines.push(line.to_owned())
            }
                
        };
        
        /*
        new_lines.push(reg.render_template(line, data).expect("Templating failed"));
            Err(ref e) if e.kind() == handlebars::RenderError => {
                new_lines.push(line.to_owned());
                io::stderr().write_all(e.as_render_error().unwrap().desc.as_bytes());
            }
        */

    }

    new_lines
}

/// Read a json data file into an arbitrary JSON dictionary.
fn read_data(filepath: &PathBuf) -> Result<serde_json::Value, Box<dyn std::error::Error>> {

    let file = File::open(filepath)?;
    let reader = std::io::BufReader::new(file);

    let data: serde_json::Value = serde_json::from_reader(reader)?;

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

    #[test]
    fn test_merge_tex() {

        let testpath = PathBuf::from("tests/data/case1/main.tex");

        let lines = merge_tex(&testpath).unwrap();

        assert_eq!(lines.len(), 13);
    }


    #[test]
    fn test_fill_data() {

        let lines: Vec<String> = vec!["Hello".into(), "I am {{years}} years old.".into(), "Goodbye.".into()];


        let data = serde_json::json!({"years": 24});


        let new_lines = fill_data(&lines, &data);

        assert_eq!(new_lines[1], "I am 24 years old.");



    }

    #[test]
    fn test_read_data() {

        let path = PathBuf::from("tests/data/case2/data.json");

        let data = read_data(&path).unwrap();

        assert_eq!(data.get("year").unwrap(), 2000);
        assert_eq!(data.get("year_str").unwrap(), "two thousand");

        let lines: Vec<String> = vec!["The year was once {{year}}".into(), "This package is called {{package_name}}.".into()];

        let new_lines = fill_data(&lines, &data);

        assert_eq!(new_lines[0], "The year was once 2000");
        assert_eq!(new_lines[1], "This package is called manus.")
    }
}
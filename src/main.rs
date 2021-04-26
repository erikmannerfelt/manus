use clap::{App, Arg};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

mod io;
mod templates;

fn main() -> std::io::Result<()> {
    match parse_cli_args() {
        Ok(x) => std::io::stdout().write_all(x.as_bytes())?,
        Err(x) => {
            std::io::stderr().write_all(x.as_bytes())?;
            std::process::exit(1)
        }
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
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbose")
                .multiple(true)
                .takes_value(false)
                .global(true)
                .about("Print non-error messages. -vv is more verbose."),
        )
        // Create the 'build' subcommand for building pdfs.
        .subcommand(
            App::new("build")
                .about("Render the manuscript with tectonic.")
                .arg(
                    Arg::new("INPUT")
                        .about("The input root tex file.")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("OUTPUT")
                        .about("The output pdf path. Defaults to the current directory.")
                        .required(false)
                        .index(2),
                )
                .arg(Arg::new("DATA").about("Data file").short('d').long("data"))
                .arg(
                    Arg::new("KEEP_INTERMEDIATES")
                        .about("Keep intermediate files.")
                        .short('k')
                        .long("keep-intermediates"),
                )
                .arg(
                    Arg::new("SYNCTEX")
                        .about("Generate synctex data")
                        .short('s')
                        .long("synctex"),
                ),
        )
        .subcommand(
            App::new("convert")
                .about("Convert to different formats.")
                .arg(
                    Arg::new("INPUT")
                        .about("The input root tex file")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("DATA")
                        .about("Data file")
                        .short('d')
                        .long("data")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("FORMAT")
                        .about("Format. Choices: [tex]. Defaults to tex.")
                        .short('f')
                        .long("format"),
                ),
        )
        .subcommand(
            App::new("merge").about("Merge 'input' clauses.").arg(
                Arg::new("INPUT")
                    .about("The input root tex file.")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    // Parse the verbosity setting. 0 is none, 1 is verbose, 2 is verybose (hehe)
    let verbosity = match matches.occurrences_of("verbosity") {
        x if x < 3 => x,
        x => return Err(format!("Invalid verbosity level: {}. Max: 2", x)),
    };

    // 'build' subcommand parser.
    if let Some(ref matches) = matches.subcommand_matches("build") {
        // Parse the filepath.
        let path_str = matches
            .value_of("INPUT")
            .expect("It's a required argument so this shouldn't fail.");

        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e)),
        };

        // Read and merge all tex files.
        let lines = match merge_tex(&filepath) {
            Ok(l) => l,
            Err(e) => return Err(format!("{:?}", e)),
        };

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

        let keep_intermediates = matches.is_present("KEEP_INTERMEDIATES");
        let synctex = matches.is_present("SYNCTEX");

        // Render the PDF
        match run_tectonic(
            &lines.join("\n"),
            &pdf_filepath,
            verbosity > 0,
            keep_intermediates,
            synctex) {
            Ok(_) => (),
            Err(_) if verbosity == 0 => return Err("Tectonic exited with an error. Run the command with --verbose to find out what went wrong.".into()),
            Err(_) => ()
        }
    }

    // 'convert' subcommand parser
    if let Some(ref matches) = matches.subcommand_matches("convert") {
        // Parse the input.
        let path_str = matches
            .value_of("INPUT")
            .expect("It's a reqired argument so this won't fail.");

        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e)),
        };

        // Write the result to stdout if it worked or the error to stderr if it didn't.
        let mut lines = match merge_tex(&filepath) {
            Ok(lines) => lines,
            Err(message) => return Err(format!("{:?}", message)),
        };

        // Parse the data argument and do template filling in case it was given.
        if let Some(data_path_str) = matches.value_of("DATA") {
            let data_path = match parse_filepath(&data_path_str, Some("json")) {
                Ok(fp) => fp,
                Err(e) => return Err(format!("{:?}", e)),
            };

            let data = match io::read_data(&data_path) {
                Ok(d) => d,
                Err(e) => return Err(format!("{:?}", e)),
            };

            lines = templates::fill_data(&lines, &data);
        }

        // Return the text to write to stdout.
        return Ok(lines.join("\n"));
    }

    // 'merge' subcommand parser.
    if let Some(ref matches) = matches.subcommand_matches("merge") {
        // Parse the input path..
        let path_str = matches
            .value_of("INPUT")
            .expect("It's a reqired argument so this won't fail.");

        // Check that the file exists and return a valid PathBuf.
        let filepath = match parse_filepath(&path_str, Some("tex")) {
            Ok(fp) => fp,
            Err(e) => return Err(format!("{:?}", e)),
        };

        // Write the result to stdout if it worked or the error to stderr if it didn't.
        match merge_tex(&filepath) {
            Ok(lines) => return Ok(lines.join("\n")),
            Err(message) => return Err(format!("{:?}", message)),
        };
    }

    // If no return statements were reached. Write an empty string to stderr.
    Err("".into())
}

/// Run tectonic to generate an output file.
fn run_tectonic(
    tex_string: &str,
    output_path: &Path,
    verbose: bool,
    keep_intermediates: bool,
    synctex: bool,
) -> tectonic::errors::Result<()> {
    // START: Tectonic black magic (basically copied from tectonic/src/lib.rs).
    let mut status = tectonic::status::NoopStatusBackend::default();

    let auto_create_config_file = false;
    let config = tectonic::ctry!(tectonic::config::PersistentConfig::open(auto_create_config_file);
                       "failed to open the default configuration file");

    let only_cached = false;
    let bundle = tectonic::ctry!(config.default_bundle(only_cached, &mut status);
                       "failed to load the default resource bundle");

    let format_cache_path = tectonic::ctry!(config.format_cache_path();
                                  "failed to set up the format cache");

    let mut files = {
        // Looking forward to non-lexical lifetimes!
        let mut sb = tectonic::driver::ProcessingSessionBuilder::default();
        sb.bundle(bundle)
            .primary_input_buffer(tex_string.as_bytes())
            .tex_input_name("texput.tex")
            .format_name("latex")
            .format_cache_path(format_cache_path)
            .keep_logs(false)
            .keep_intermediates(keep_intermediates)
            .print_stdout(verbose)
            .synctex(synctex)
            .output_format(tectonic::driver::OutputFormat::Pdf)
            .do_not_write_output_files();

        let mut sess = tectonic::ctry!(sb.create(&mut status); "failed to initialize the LaTeX processing session");
        tectonic::ctry!(sess.run(&mut status); "the LaTeX engine failed");
        sess.into_file_data()
    };
    // END: Tectonic black magic.

    // Find the pdf in the tectonic output and return its data.
    let file_data = match files.remove(&std::ffi::OsString::from(&"texput.pdf")) {
        Some(file) => file.data,
        None => {
            return Err(tectonic::errmsg!(
                "LaTeX didn't report failure, but no PDF was created (??)"
            ))
        }
    };
    // Create a new file and write the PDF data to it.
    let mut file = File::create(&output_path).expect("");
    file.write_all(&file_data).expect("");

    // If keep_intermediates was provided, loop over all of them and save them beside the pdf.
    // If only synctex was given, reuse the same loop but skip all files except the synctex file.
    if keep_intermediates | synctex {
        for (filename_os, data) in files {
            let filename = PathBuf::from(filename_os);
            // Strip the extension. In the case of synctex, its conversion is hardcoded..
            let extension = match filename == std::ffi::OsString::from(&"texput.synctex.gz") {
                true => std::ffi::OsString::from("synctex.gz"),
                false => match filename.extension() {
                    Some(x) => x.to_os_string(),
                    None => continue,
                },
            };
            // If keep_intermediates is false, only the synctex file should be written.
            if !keep_intermediates & (extension != std::ffi::OsString::from("synctex.gz")) {
                continue;
            };
            let mut path = PathBuf::from(output_path.file_stem().unwrap());
            path.set_extension(extension);

            // Create a new file and write the PDF data to it.
            let mut file = File::create(&path)
                .unwrap_or_else(|_| panic!("Could not open {} to write", path.to_str().unwrap()));
            file.write_all(&data.data)
                .unwrap_or_else(|_| panic!("Could not write to {}.", path.to_str().unwrap()));
        }
    }

    Ok(())
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
fn parse_filepath(
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

/// Read a tex file and recursively merge all \\input{} statements.
///
/// # Arguments
/// * `filepath`: A relative or absolute path to the main.tex.
fn merge_tex(filepath: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Create the output line vector
    let mut lines: Vec<String> = Vec::new();

    // Parse the lines of the main file.
    let main_lines = io::read_tex(&filepath)?;

    // Loop over the lines and handle any \input clauses.
    for line in main_lines {
        // If it doesn't contain and input, just continue.
        if !line.contains(r"\input{") {
            lines.push(line);
            continue;
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
}

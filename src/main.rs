//! A **manus**cript helper to simplify writing good papers.
//!
//! ## Simple CLI usage
//!
//! ```bash
//! manus build main.tex  # This will build a main.pdf
//! ```
//! will use [tectonic](https://github.com/tectonic-typesetting/tectonic) to build the file called
//! `main.tex`, with some slightly improved error messages.
//!
//! When submitting manuscripts to academic journals, there is often a requirement to only have one
//! source `TeX` file. If the manuscript is written with e.g. multiple chapters
//! (`\input{introduction.tex}` etc.), this can be merged easily with `manus`:
//! ```bash
//! manus merge main.tex > merged_text.tex
//! ```
//!
//! ## Templating
//! The most promiment functionality of `manus` is bridging `TeX` and
//! [handlebars](https://handlebarsjs.com/); a powerful templating system to separate text and
//! data.
//!
//! ### Pure-LaTeX example
//! ```tex
//! \documentclass{article}
//!
//! \begin{document}
//! We used 85819 separate measurements to find the value of 58.242$\pm$0.011 units.
//! \end{document}
//! ```
//! Note that data and text are quite easily separated here.
//! Imagine, however, a more complex example of ten pages of text and numbers, and a reviewer's
//! evil comments suddenly called for revision of half the numbers in the manuscript.
//! How will you know that you managed to change all of them!?
//!
//! ### Templating example
//! In `manus` the above example would consist of two files; one for data and one for text.
//!
//! The data file can be called `data.toml`:
//! ```toml
//! n_measurements = 85819  # This is how many measurements we have right now, but it may change!
//!
//! resultant_value = 58.242   # The value (which might change?)
//! resultant_value_pm = 0.011  # The error of the value
//! ```
//! And the text; `main.tex`:
//! ```tex
//! \documentclass{article}
//!
//! \begin{document}
//! We used {{n_measurements}} separate measurements to find the value of {{pm resultant_value}}
//! units.
//! \end{document}
//! ```
//! An can be built with:
//! ```bash
//! manus build -d data.toml main.tex  # This will build a main.pdf
//! ```
//!
//! Now, we have moved all of our data to a separate machine-readable file.
//! This has many implications:
//! 1. Data are easily revised throughout the text, so updating results along the way is simple.
//! 2. The supported data formats (JSON and TOML) are machine-readable, meaning they can be created
//!    automatically from any script written in python, rust, julia etc. "Hardcoding" values can
//!    theoretically be avoided completely!
//! 3. (See below) Helpers can reduce data repetition by doing simple arithmetic and/or formatting
//!    for you.
//!
//!
//! ### Template helpers
//!
//! #### pm --- plus-minus
//! Arguments:
//! * `decimal`: Optional. The decimal to round both values to.
//! * `key`: The key to print and find a corresponding `_pm` key for.
//! 
//! As you saw in the example above, the `n_measurements` key could be fetched from the data file
//! by simply writing `{{n_measurements}}` in the `TeX` file.
//! The `resultant_value` has an associated error (could have been called `resultant_error`),
//! whereby we would write `{{resultant_value}}$\pm${{resultant_error}}`.
//! This is quite repetitive, however, so a helper `pm` exists to simplify this. 
//! 
//! If `{{pm anykey}}` is written, the helper will look for an associated error key: `anykey_pm`.
//! In the case above, this would be `resultant_value_pm`.
//!
//! 
//! If we want to round both the value and its error, the `decimal` optional argument can be used:
//!
//! ```tex
//! {{pm 2 resultant_value}}
//! ```
//! renders to:
//!
//! ```tex
//! 58.24$\pm$0.01
//! ```
//!
//! #### round --- Round a value to the nearest decimal
//! Arguments:
//! * `decimal`: Optional. The decimal to round a value to. Defaults to 0 (integer)
//! * `value`: The value to round
//!
//! ```tex
//! {{round resultant_value}}
//! {{round 2 resultant_value}}
//! {{round -1 resultant_value}}  % This will round upwards (to the nearest 10)
//! ```
//! renders to:
//! ```tex
//! 58
//! 58.24
//! 60
//! ```
//!
//! #### roundup --- Round a value to the nearest power of ten
//! Arguments:
//! * `power`: Optional. The power of ten to round toward. Defaults to 0 (integer)
//! * `value`: The value to round
//!
//! `roundup` is the same as `round`, only with an inverted sign.
//!
//! ```tex
//! {{roundup resultant_value}}
//! {{roundup 1 resultant_value}}
//! {{roundup -1 resultant_value}}  % This will round downwards (to the nearest decimal)
//! ```
//! renders to:
//! ```tex
//! 58
//! 60
//! 58.2
//! ```
//!
//! #### sep --- Add thousand-separators around large numbers
//! Arguments:
//! * `value`: The value to make more readable.
//!
//! **Requires** a key in the data called `separator` which will be used to separate the values.
//!
//! With `separator = '\,'` (a comma-sized whitespace):
//!
//! ```tex
//! {{sep n_measurements}}
//! ```
//! renders to:
//! ```tex
//! 85\,819
//! ```
//! which looks approximately like '85 819' when rendered into the PDF.
//! 
//!
//! ## Expressions
//! The "in-`TeX`" helpers are great for small one-time formatting, but expressions in `manus` take
//! the next step.
//!
//! Writing `"expr: "` as a value in the data file will evaluate that expression before rendering.
//!
//! With a `data.toml`:
//! ```toml
//! n_total_snacks = 2042
//! n_eaten_snacks = 1567
//!
//! n_remaining_snacks = "expr: n_total_snacks - n_eaten_snacks"
//! n_remaining_percentage = "expr: round(100 * n_remaining_snacks / n_total_snacks)"
//! ```
//! Since `n_eaten_snacks` and `n_remaining_snacks` are always related to each other, and they will change if we eat one more, it's great to define one as a function of the other, instead of "hardcoding" both.
//!
//! Note that `n_remaining_percentage` depends on an expression (`n_remaining_snacks`), which
//! is solved by recursively evaluating the independent expressions first, before the
//! dependent expressions.
//! If two expressions are dependent on each other (a circular dependency), this will raise a
//! descriptive recursion error.
//!
//! ### Expression functions
//! **NOTE**: As of right now (26 May 2021), the underyling expression evaluation engine cannot
//! understand negative signs properly. `-1` needs to be written as `0-1`, unfortunately! Hopefully
//! this will change soon.
//!
//! #### round
//! Arguments:
//! * `value`: The value to round.
//! * `decimal`
//!
//!
//! ## Conversions
//!
//! Converting a `manus`-flavoured `TeX` into pure `TeX` is done simply:
//!
//! ```bash
//! manus convert main.tex > boring_version_of_main.tex
//! ```
//! where the `--format` argument implicitly defaults to `tex`.
//!
//!
//!
//! ## Advanced: Piping
//!
//! For advanced users, the concept of UNIX piping is embraced with `manus`.
//!
//! The `-` symbol is used for specifying what to read from stdin:
//! ```bash
//! curl https://example.com/my_json_data | manus build --data - main.tex
//! ```
//! Currently, only JSON is supported from pipes.
//!
//! If, for some reason, we want to use another compiler than `tectonic`, we can pipe the converted
//! `tex` text data to it:
//! ```bash
//! manus convert --data=data.toml main.tex | another_tex_compiler
//! ```
//!
//!
use clap::{App, Arg};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

mod io;
mod templates;

fn main() -> std::io::Result<()> {
    match parse_cli_args() {
        Ok(x) => {
            std::io::stdout().write_all(x.as_bytes())?;
            std::process::exit(0)
        }
        Err(x) => {
            std::io::stderr().write_all(x.as_bytes())?;
            std::process::exit(1)
        }
    };
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
                        .about("The input root tex file. If '-', read from stdin.")
                        .required(true),
                )
                .arg(
                    Arg::new("OUTPUT")
                        .about("The output pdf path. Defaults to the current directory.")
                        .required(false),
                )
                .arg(
                    Arg::new("DATA")
                        .about("Data filepath. If '-', read from stdin.")
                        .short('d')
                        .long("data")
                        .takes_value(true),
                )
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
                        .about("The input root tex file. If '-', read from stdin.")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("DATA")
                        .about("Data filepath. If '-', read from stdin.")
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

        // Try to read the lines from the path (or stdin) and return the given (or appropriate if not given) pdf filepath
        let (mut lines, pdf_filepath) =
            match io::get_lines_and_output_path(path_str, matches.value_of("OUTPUT")) {
                Ok(x) => x,
                Err(e) => return Err(e.to_string()),
            };

        // Fill the data if a data path was given.
        if let Some(datafile) = matches.value_of("DATA") {
            // If both the datafile and path_str was -, raise an error.
            if (datafile.trim() == "-") & (path_str.trim() == "-") {
                return Err("Input tex and data cannot both be from stdin.".into());
            };
            let data = match io::get_data_from_str(&datafile) {
                Ok(v) => v,
                Err(e) => return Err(e.to_string()),
            };

            lines = templates::fill_data(&lines, &data)?;
        };

        let keep_intermediates = matches.is_present("KEEP_INTERMEDIATES");
        let synctex = matches.is_present("SYNCTEX");

        if let Some(parent) = pdf_filepath.parent() {
            if !parent.is_dir() & !parent.to_str().unwrap().is_empty() {
                return Err(format!(
                    "Parent directory '{}' does not exist",
                    parent.to_str().unwrap()
                ));
            }
        }
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
        };

        return Ok("".into());
    }

    // 'convert' subcommand parser
    if let Some(ref matches) = matches.subcommand_matches("convert") {
        // Parse the input.
        let path_str = matches
            .value_of("INPUT")
            .expect("It's a reqired argument so this won't fail.");

        // Try to read the lines from the path (or stdin) and return the given (or appropriate if not given) pdf filepath
        let (mut lines, _) =
            match io::get_lines_and_output_path(path_str, matches.value_of("OUTPUT")) {
                Ok(x) => x,
                Err(e) => return Err(e.to_string()),
            };

        // Fill the data if a data path was given.
        if let Some(datafile) = matches.value_of("DATA") {
            // If both the datafile and path_str was -, raise an error.
            if (datafile.trim() == "-") & (path_str.trim() == "-") {
                return Err("Input tex and data cannot both be from stdin.".into());
            };
            let data = match io::get_data_from_str(&datafile) {
                Ok(v) => v,
                Err(e) => return Err(e.to_string()),
            };

            lines = templates::fill_data(&lines, &data)?;
        };

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
        let filepath = match io::parse_filepath(&path_str, Some("tex")) {
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

            // If the output path has a parent, append this to the filename.
            if let Some(parent) = output_path.parent() {
                path = parent.join(path);
            }

            // Create a new file and write the PDF data to it.
            let mut file = File::create(&path)
                .unwrap_or_else(|_| panic!("Could not open {} to write", path.to_str().unwrap()));
            file.write_all(&data.data)
                .unwrap_or_else(|_| panic!("Could not write to {}.", path.to_str().unwrap()));
        }
    }

    Ok(())
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
    let mut i = 0;
    for line in main_lines {
        // If it doesn't contain and input, just continue.
        if !line.contains(r"\input{") {
            lines.push(line);
            i += 1;
            continue;
        }
        let mut trimmed_line = line[(line.find(r"\input{").unwrap() + 7)..].to_owned();
        trimmed_line = trimmed_line[..trimmed_line
            .find('}')
            .unwrap_or_else(|| panic!("Unclosed delimiter at line {}", i))]
            .to_owned();
        let mut input_path = PathBuf::from(trimmed_line);

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
        i += 1;
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_merge_tex() {
        let testpath = PathBuf::from("tests/data/case1/main.tex");

        let lines = merge_tex(&testpath).unwrap();

        assert_eq!(lines.len(), 13);
    }
}

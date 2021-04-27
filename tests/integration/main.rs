#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*; // Add methods on commands
    use predicates::prelude::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn test_merge() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("manus")?;

        cmd.arg("merge").arg("tests/data/case1/main.tex");

        // Check that it succeeded and that the concusions.tex was successfully merged.
        cmd.assert().success().stdout(predicate::str::contains(
            "I conclude that the premise must be true",
        ));

        Ok(())
    }

    #[test]
    fn test_convert() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("manus")?;

        cmd.arg("convert")
            .arg("--format=tex")
            .arg("--data=tests/data/case2/data.json")
            .arg("tests/data/case2/main.tex");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains(
                "The year 2000 can be called the two thousands.",
            ))
            .stdout(predicate::str::contains("This is written from Norway."))
            .stdout(predicate::str::contains("With two decimals, it is: 3.14"));

        let mut cmd = Command::cargo_bin("manus")?;

        cmd.arg("convert")
            .arg("--format=tex")
            .arg("--data=tests/data/case3/data.json")
            .arg("tests/data/case3/main.tex");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("mean change of 1.3$\\pm$0.5 m"));

        Ok(())
    }

    #[test]
    fn test_verbosity() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("manus")?;

        cmd.arg("-vvv");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Invalid verbosity level"));

        Ok(())
    }

    #[test]
    fn test_build() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;

        // Create some sample tex to try to render.
        let tex_string = r#"
        \documentclass{article}
        \begin{document}
        Hello there!
        \end{document}
        "#;

        let tex_path = temp_dir.path().join("main.tex");
        let output_path = temp_dir.path().join("main.pdf");

        {
            let mut tex_file = std::fs::File::create(&tex_path)?;
            tex_file.write(tex_string.as_bytes())?;
        }

        let mut cmd = Command::cargo_bin("manus")?;

        cmd.arg("build")
            .arg("-v")
            .arg("--keep-intermediates")
            .arg(tex_path.to_str().unwrap())
            .arg(output_path.to_str().unwrap());

        cmd.assert().success();

        let expected_files = vec![
            output_path,
            temp_dir.path().join("main.aux"),
            temp_dir.path().join("main.log"),
        ];

        for file in expected_files {
            if !file.is_file() {
                panic!(
                    "{:?} did not exist. {:?}",
                    file,
                    std::fs::read_dir(temp_dir).unwrap()
                );
            }
        }

        // Try piping tex code and see if a pdf was generated.
        let output_path2 = temp_dir.path().join("main2.pdf");
        let mut cmd2 = Command::cargo_bin("manus")?
            .arg("build")
            .arg("-")
            .stdin(Stdio::piped())
            .arg(&output_path2)
            .spawn()?;

        // Write the tex to the stdin.
        {
            let stdin = cmd2.stdin.as_mut().expect("failed to get stdin");
            stdin.write_all(tex_string.as_bytes())?;
        }

        // Wait for the command to exit.
        cmd2.wait()?;

        // Check that the pdf exists.
        assert!(
            output_path2.is_file(),
            "Output from piping tex did not exist."
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*; // Add methods on commands
    use predicates::prelude::*;
    use std::process::Command; // Used for writing assertions

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
            .stdout(predicate::str::contains("This is written from Norway."));

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
}

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
}

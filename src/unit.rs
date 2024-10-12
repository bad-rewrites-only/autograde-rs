use std::str::{from_utf8, Utf8Error};

use log::debug;
use miette::{Diagnostic, Report, SourceSpan};
use serde::Deserialize;
use similar::{ChangeTag, DiffOp, TextDiff};
use thiserror::Error;
use tokio::process::Command;

use crate::config::Config;

// use crate::build::BuildSystem;

#[derive(Deserialize, Debug)]
pub struct TestUnits {
    pub tests: Vec<TestUnit>,
}

#[derive(Deserialize, Debug)]
pub struct TestUnit {
    name: String,
    input: Vec<String>,
    expected: String,
    rubric: u64,
}

impl TestUnit {
    /// Interpolate strings with '$' place holder
    pub fn interpolate_config(
        &mut self,
        config: &Config,
        executable: &str,
    ) -> Result<(), UnitError> {
        const PROJECT_DIR_SUBSTRING: &str = "$project";
        const DIGITAL_JAR_SUBSTRING: &str = "$digital";
        self.input
            .iter_mut()
            .map(|slice| {
                if slice.contains(PROJECT_DIR_SUBSTRING) {
                    *slice = slice.replace(PROJECT_DIR_SUBSTRING, executable)
                } else if slice.contains(DIGITAL_JAR_SUBSTRING) {
                    let digital_path = config.test.clone().unwrap().digital_path();
                    *slice = match digital_path {
                        Some(path) => path,
                        None => {
                            eprintln!("Error: digital_path is not set.");
                            return Err(UnitError::DigitalJarPathNotSpecified);
                        }
                    };
                }
                Ok(())
            })
            .for_each(drop); // Consume the iterator
        Ok(())
    }
}

#[derive(Debug)]
struct UnitOutput {
    // output: String,
    name: String,
    grade: u64,
    rubric: u64,
}

#[derive(Error, Diagnostic, Debug)]
#[error("One or more tests failed")]
pub struct UnitErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<UnitError>,
}

#[derive(Error, Diagnostic, Debug)]
pub enum UnitError {
    // #[error("Exit code wasn't zero")]
    // NonZeroExit,
    #[error("Program crashed")]
    ProgramCrashed,
    // #[error(transparent)]
    // #[diagnostic(transparent)]
    // IncorrectOutput(#[from] IncorrectOutput),
    #[error("Output doesn't match expected result")]
    IncorrectOutput,
    #[error("Not UTF8")]
    NotUtf8(Utf8Error),
    #[error("Could not run program")]
    Wrapped(std::io::Error),
    #[error("Could not interpolate string")]
    DigitalJarPathNotSpecified,
}

#[derive(Error, Diagnostic, Debug)]
#[error("Output doesn't match expected result")]
// #[diagnostic(
//     help("")
// )]
#[diagnostic()]
pub struct IncorrectOutput {
    #[related]
    span_list: Vec<IncorrectSpan>,
}

#[derive(Error, Diagnostic, Debug, Clone)]
#[error("Want: {expected:?}, got: ")]
struct IncorrectSpan {
    expected: Option<String>,
    #[source_code]
    got: String,
    #[label("here")]
    at: SourceSpan,
}

// fn pull_tests() {}

// #[allow(async_fn_in_trait)]
// pub trait RunProject {
//     async fn run(self) -> miette::Result<u64>;
// }

// #[allow(async_fn_in_trait)]
// pub trait RunUnit {
//     async fn run(&self) -> Result<TestOutput, UnitError>;
// }

// impl RunProject for TestUnits {
impl TestUnits {
    pub async fn run(self) -> miette::Result<u64> {
        let mut tasks = Vec::with_capacity(self.tests.len());
        for unit in self.tests {
            tasks.push(tokio::spawn(unit.run()))
        }

        let mut outputs = Vec::with_capacity(tasks.len());
        for task in tasks {
            outputs.push(task.await.unwrap());
        }

        let grade: u64 = outputs
            .into_iter()
            .map(|out| match out {
                Ok(out) => {
                    println!("{}: ({}/{})", out.name, out.grade, out.rubric);
                    out.grade
                }
                Err(e) => {
                    let report = Report::new(e);
                    eprintln!("{:?}", report);
                    0
                }
            })
            .sum();

        Ok(grade)
    }
}

// impl RunUnit for Unit {
impl TestUnit {
    async fn run(self) -> Result<UnitOutput, UnitError> {
        let output = Command::new(self.input.first().expect("Empty input in tests file!"))
            .args(
                self.input
                    .split_first()
                    .expect("Empty input in tests file!")
                    .1,
            )
            .output()
            .await
            .map_err(UnitError::Wrapped)?;

        // TODO do we care about nonzero exits?
        // if !output.status.success() {
        // }

        let stdout = from_utf8(&output.stdout)
            .map_err(UnitError::NotUtf8)?
            .trim();

        let mut errors = vec![];
        let diff = TextDiff::from_lines(self.expected.trim(), stdout);
        for op in diff.ops() {
            for change in diff.iter_changes(op) {
                if change.tag() == ChangeTag::Equal || change.value() == "\n" {
                    continue;
                }

                // println!("{:#?}", change);

                errors.push(IncorrectSpan {
                    expected: Some(self.expected.clone()),
                    // .lines()
                    // .nth(change.old_index().unwrap())
                    // .map(|s| s.to_owned()),
                    got: change.to_string(),
                    at: (op.new_range().into()),
                })
            }
        }

        if errors.is_empty() {
            // TODO change to actual partial grading?
            Ok(UnitOutput {
                name: self.name,
                grade: self.rubric,
                rubric: self.rubric,
            })
        } else {
            Err(
                UnitError::IncorrectOutput, //     (IncorrectOutput {
                                            //     // src: stdout.into(),
                                            //     span_list: errors,
                                            // })
            )
        }
    }
}

// tokio bug https://github.com/tokio-rs/tokio/pull/6874
#[allow(clippy::needless_return)]
#[tokio::test]
async fn test_unit_run() -> miette::Result<()> {
    use miette::IntoDiagnostic;

    let test = TestUnit {
        name: "".into(),
        input: ["echo", "hello world"]
            .iter_mut()
            .map(|s| s.to_owned())
            .collect(),
        expected: "hello world".into(),
        rubric: 100,
    };
    test.run().await.into_diagnostic().unwrap();

    let test = TestUnit {
        name: "".into(),
        input: ["echo", "howdy y'all"]
            .iter_mut()
            .map(|s| s.to_owned())
            .collect(),
        expected: "hello world".into(),
        rubric: 100,
    };
    // test.run().await?;
    let res = test.run().await;
    assert!(res.is_err());
    println!("{:?}", res);

    Ok(())
}

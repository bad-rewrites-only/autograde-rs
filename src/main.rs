#![allow(unused)]

use std::env::current_dir;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use clap::{Parser, Subcommand};
use log::{debug, info};
use miette::WrapErr;

use autograde::build::make;
use autograde::config::Config;
use autograde::unit::TestUnits;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long)]
    project_path: Option<String>,
    // #[arg(short, long)]
    // tests_path: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the test case(s) for the current project
    Test {
        #[arg(short, long)]
        tests_path: Option<String>,
    },
    // Configure,
}

// tokio bug https://github.com/tokio-rs/tokio/pull/6874
#[allow(clippy::needless_return)]
#[tokio::main]
async fn main() -> miette::Result<()> {
    env_logger::init();

    match Args::parse().command {
        Command::Test { tests_path } => {
            let config = Config::read_or_create().unwrap();
            let config_test = config
                .test
                .clone()
                .context("Config file missing test section!")
                .unwrap();

            let tests_path = if let Some(u) = tests_path {
                u
            } else {
                config_test
                    .tests_path
                    .clone()
                    .context("Could not find test_path in config file!")
                    .unwrap()
            };

            let digital_path = config_test.digital_path();
            info!("Digital JAR path: {:?}", digital_path);

            // make().await?;

            // `foo-bar` project directory implies `bar` executable name
            let project_dir = current_dir().unwrap();
            let project_exec = project_dir
                .file_name()
                .unwrap()
                .to_str()
                .map(|s| s.split('-').next().unwrap_or(s))
                .expect("EXECUTABLE NAME");
            info!("project executable name: {}", project_exec);

            // TODO support tilde expansion
            // TODO search pwd/parents for tests dir
            let mut tests_path = PathBuf::from_str(&tests_path)
                .with_context(|| format!("Invalid path! {}", tests_path))
                .unwrap();
            tests_path.push(project_exec);
            tests_path.push(project_exec);
            tests_path.set_extension("toml");
            info!("test path: {:?}", tests_path);

            // TODO move to tests.rs
            let tests_file = read_to_string(&tests_path).unwrap();
            let mut tests: TestUnits = toml::from_str(&tests_file)
                .with_context(|| format!("Could not parse tests at {}!", tests_path.display()))
                .unwrap();

            tests.tests.iter_mut().for_each(|test| {
                match test.interpolate_config(&config, project_exec) {
                    Ok(_) => {
                        info!("Interpolation succeeded!");
                    }
                    Err(e) => {
                        panic!("Interpolation failed: {}", e);
                    }
                }
            });
            debug!(
                "parsed and interpolated test units:
                {:#?}",
                tests
            );

            // TODO auto pull
            let grade = tests.run().await?;
            info!("grade: {}", grade);
        }
    }

    Ok(())
}

use std::str::from_utf8;

use anyhow::{bail, Context};
use serde::Deserialize;
use tokio::process::Command;

#[derive(Deserialize, Debug)]
pub enum BuildSystem {
    Make,
    Digital,
    Cargo,
}

pub async fn make() -> anyhow::Result<()> {
    // TODO just, cargo, etc
    let make_output = Command::new("make").output().await.with_context(|| {
        "Could not spawn a child proccess or get its output!
        Tried to call: make"
            .to_string()
    })?;

    if !make_output.status.success() {
        eprintln!("Failed to make!");
        let make_stdout = from_utf8(&make_output.stdout)?;
        let make_stderr = from_utf8(&make_output.stderr)?;
        println!("{}", make_stdout);
        eprintln!("{}", make_stderr);

        bail!("Make failed with {}", make_output.status)
    }

    Ok(())
}

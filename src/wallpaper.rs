use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub fn set_wallpaper(image_path: &Path) -> Result<()> {
    let path_str = image_path
        .to_str()
        .context("Image path is not valid UTF-8")?;

    let script = format!(
        r#"tell application "System Events" to tell every desktop to set picture to POSIX file "{path_str}""#
    );

    let status = Command::new("osascript")
        .args(["-e", &script])
        .status()
        .context("Failed to run osascript")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("osascript exited with {status}")
    }
}

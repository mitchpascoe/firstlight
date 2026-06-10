use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub fn set_wallpaper(image_path: &Path) -> Result<()> {
    let path_str = image_path
        .to_str()
        .context("Image path is not valid UTF-8")?;

    // Directly edit the wallpaper plist so all spaces on all displays get updated,
    // then restart WallpaperAgent to pick up the change.
    let script = r#"
import plistlib, os
from pathlib import Path

path = os.environ['WALLPAPER_PATH']
plist_path = Path.home() / 'Library/Application Support/com.apple.wallpaper/Store/Index.plist'

with open(plist_path, 'rb') as f:
    plist = plistlib.load(f)

config = plistlib.dumps(
    {'type': 'imageFile', 'url': {'relative': 'file://' + path}},
    fmt=plistlib.FMT_BINARY
)
choice = {'Configuration': config, 'Files': [], 'Provider': 'com.apple.wallpaper.choice.image'}

def update(obj):
    if isinstance(obj, dict):
        if obj.get('Type') == 'idle':
            return
        if 'Choices' in obj and 'Shuffle' in obj:
            obj['Choices'] = [choice]
        else:
            for v in obj.values():
                update(v)
    elif isinstance(obj, list):
        for item in obj:
            update(item)

update(plist)

with open(plist_path, 'wb') as f:
    plistlib.dump(plist, f, fmt=plistlib.FMT_BINARY)

os.system('killall WallpaperAgent 2>/dev/null')
"#;

    let status = Command::new("python3")
        .args(["-c", script])
        .env("WALLPAPER_PATH", path_str)
        .status()
        .context("Failed to run python3")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("set_wallpaper failed with {status}")
    }
}

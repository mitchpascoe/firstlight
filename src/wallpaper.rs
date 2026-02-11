use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result};
use plist::{Dictionary, Value};

pub fn set_wallpaper(image_path: &Path) -> Result<()> {
    let plist_path = dirs::home_dir()
        .context("No home directory")?
        .join("Library/Application Support/com.apple.wallpaper/Store/Index.plist");

    let config_data = build_config_data(image_path)?;
    let mut plist = Value::from_file(&plist_path).context("Failed to read wallpaper plist")?;

    let now: plist::Date = SystemTime::now().into();

    if let Some(root) = plist.as_dictionary_mut() {
        if let Some(Value::Dictionary(displays)) = root.get_mut("Displays") {
            for display in displays.values_mut() {
                update_entry(display, &config_data, &now);
            }
        }

        if let Some(Value::Dictionary(spaces)) = root.get_mut("Spaces") {
            for space in spaces.values_mut() {
                let Some(space) = space.as_dictionary_mut() else {
                    continue;
                };
                if let Some(default) = space.get_mut("Default") {
                    update_entry(default, &config_data, &now);
                }
                if let Some(Value::Dictionary(displays)) = space.get_mut("Displays") {
                    for display in displays.values_mut() {
                        update_entry(display, &config_data, &now);
                    }
                }
            }
        }

        if let Some(sys_default) = root.get_mut("SystemDefault") {
            update_entry(sys_default, &config_data, &now);
        }
    }

    plist
        .to_file_binary(&plist_path)
        .context("Failed to write wallpaper plist")?;

    Command::new("killall")
        .arg("WallpaperAgent")
        .stderr(std::process::Stdio::null())
        .status()
        .ok();

    Ok(())
}

fn build_config_data(image_path: &Path) -> Result<Value> {
    let path_str = image_path
        .to_str()
        .context("Image path is not valid UTF-8")?;
    let url_str = format!("file://{path_str}");

    let mut url_dict = Dictionary::new();
    url_dict.insert("relative".to_string(), Value::String(url_str));

    let mut config = Dictionary::new();
    config.insert("type".to_string(), Value::String("imageFile".to_string()));
    config.insert("url".to_string(), Value::Dictionary(url_dict));

    let config_value = Value::Dictionary(config);
    let mut buf = Vec::new();
    config_value.to_writer_binary(&mut buf)?;

    Ok(Value::Data(buf))
}

fn update_entry(entry: &mut Value, config_data: &Value, now: &plist::Date) {
    let Some(entry) = entry.as_dictionary_mut() else {
        return;
    };

    let Some(desktop) = entry.get_mut("Desktop").and_then(|v| v.as_dictionary_mut()) else {
        return;
    };

    let Some(content) = desktop.get_mut("Content").and_then(|v| v.as_dictionary_mut()) else {
        return;
    };

    let Some(choices) = content.get_mut("Choices").and_then(|v| v.as_array_mut()) else {
        return;
    };

    if let Some(choice) = choices.first_mut().and_then(|v| v.as_dictionary_mut()) {
        choice.insert("Configuration".to_string(), config_data.clone());
        choice.insert(
            "Provider".to_string(),
            Value::String("com.apple.wallpaper.choice.image".to_string()),
        );
    }

    desktop.insert("LastSet".to_string(), Value::Date(*now));
    desktop.insert("LastUse".to_string(), Value::Date(*now));
}

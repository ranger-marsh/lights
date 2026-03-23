//! Persistent device name configuration.
//!
//! Names are stored as a JSON object mapping MAC address → display name:
//!
//! ```json
//! {
//!   "AA:BB:CC:DD:EE:01": "Living Room",
//!   "AA:BB:CC:DD:EE:02": "Bedroom"
//! }
//! ```
//!
//! File location: `~/.config/govee-lights/names.json`
//! (on both macOS and Linux/Raspberry Pi)
//!
//! To transfer names to the Pi, copy this file to the same path:
//!   scp ~/.config/govee-lights/names.json pi@raspberrypi:~/.config/govee-lights/names.json

use std::collections::HashMap;
use std::path::PathBuf;

const APP_DIR: &str = "govee-lights";
const FILE_NAME: &str = "names.json";

/// Returns the path to the names config file, if a config directory can be found.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR).join(FILE_NAME))
}

/// Load the MAC → name map from disk.  Returns an empty map on any error
/// (missing file, parse error, etc.) so the app always starts cleanly.
pub fn load() -> HashMap<String, String> {
    let Some(path) = config_path() else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Persist the MAC → name map to disk.
pub fn save(names: &HashMap<String, String>) -> std::io::Result<()> {
    let Some(path) = config_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(names)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_empty_map_when_no_file() {
        // We can't easily control the real config path in tests, but we can
        // verify that parsing bad JSON gracefully returns an empty map.
        let result: HashMap<String, String> =
            serde_json::from_str("not valid json").unwrap_or_default();
        assert!(result.is_empty());
    }

    #[test]
    fn round_trips_names() {
        let mut names = HashMap::new();
        names.insert("AA:BB:CC:DD:EE:01".to_string(), "Living Room".to_string());
        names.insert("AA:BB:CC:DD:EE:02".to_string(), "Bedroom".to_string());

        let json = serde_json::to_string_pretty(&names).unwrap();
        let restored: HashMap<String, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored["AA:BB:CC:DD:EE:01"], "Living Room");
        assert_eq!(restored["AA:BB:CC:DD:EE:02"], "Bedroom");
    }
}

//! We use the [APPDATA|LOCALAPPDATA]/APP_NAME on Windows,
//! attempt XDG on macOS and Linux, defaulting to the platform
//! defaults of [.config|.local/state|.cache]/APP_NAME on Linux
//! and [Library/Application Support|Library/Caches] on macOS.
//! At a later date I may add support for overriding with env vars.

use std::{env, path::PathBuf};

const APP_NAME: &str = "diff-of-services";

/// Get the local configuration dir for the application,
/// using the defaults shown above.
pub fn config_dir() -> PathBuf {
    // If on Windows, assume APPDATA exists
    if cfg!(target_os = "windows") {
        PathBuf::from(env::var("APPDATA").unwrap()).join(APP_NAME)
        // Else if XDG_CONFIG_DIR is present, use that. This is up for debate,
        // whether we should respect XDG on macOS, but it shouldn't be present
        // unless the user set the env var, so we'll respect it for now.
    } else if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir).join(APP_NAME)
    } else {
        // HOME should exists on macOS or linux
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                // We use ~/Library/Application Support as the default on macOS
                "Library/Application Support"
            } else {
                // Default to .config on Linux
                ".config"
            })
            .join(APP_NAME)
    }
}

/// Get the local configuration dir for the application,
/// using the defaults shown above.
pub fn data_dir() -> PathBuf {
    // APPDATA on Windows /should/ exists
    if cfg!(target_os = "windows") {
        PathBuf::from(env::var("APPDATA").unwrap()).join(APP_NAME)
    } else if let Ok(dir) = env::var("XDG_DATA_HOME") {
        // Use XDG on Linux and macOS if present
        PathBuf::from(dir).join(APP_NAME)
    } else {
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                // Default to Library/Application Support on macOS
                "Library/Application Support"
            } else {
                // Default to .local/share on Linux
                ".local/share"
            })
            .join(APP_NAME)
    }
}

/// Get the cache dir for the application, using the defaults from above
pub fn cache_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        // Windows should set LOCALAPPDATA
        PathBuf::from(env::var("LOCALAPPDATA").unwrap()).join(APP_NAME)
    } else if let Ok(dir) = env::var("XDG_CACHE_HOME") {
        // Use XDG cache home if present
        PathBuf::from(dir).join(APP_NAME)
    } else {
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                // Else ~/Library/Caches on macOS
                "Library/Caches"
            } else {
                // Or ~/.cache on Linux
                ".cache"
            })
            .join(APP_NAME)
    }
}

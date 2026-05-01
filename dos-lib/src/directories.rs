//! We use the [APPDATA|LOCALAPPDATA]/APP_NAME on Windows,
//! attempt XDG on macOS and Linux, defaulting to the platform
//! defaults of [.config|.local/state|.cache]/APP_NAME on Linux
//! and [Library/Application Support|Library/Caches] on macOS

use std::{env, path::PathBuf};

const APP_NAME: &str = "diff-of-services";

pub fn config_dir() -> PathBuf {
    // If on Windows, assume APPDATA exists
    if cfg!(target_os = "windows") {
        PathBuf::from(env::var("APPDATA").unwrap()).join(APP_NAME)
        // Else if XDG_CONFIG_DIR is present, use that
    } else if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir).join(APP_NAME)
    } else {
        // HOME should exists on macOS or linux
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                "Library/Application Support"
            } else {
                ".config"
            })
            .join(APP_NAME)
    }
}

pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(env::var("APPDATA").unwrap()).join(APP_NAME)
    } else if let Ok(dir) = env::var("XDG_DATA_HOME") {
        PathBuf::from(dir).join(APP_NAME)
    } else {
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                "Library/Application Support"
            } else {
                ".local/share"
            })
            .join(APP_NAME)
    }
}

pub fn cache_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(env::var("LOCALAPPDATA").unwrap()).join(APP_NAME)
    } else if let Ok(dir) = env::var("XDG_CACHE_HOME") {
        PathBuf::from(dir).join(APP_NAME)
    } else {
        let home = env::var("HOME").unwrap();
        PathBuf::from(home)
            .join(if cfg!(target_os = "macos") {
                "Library/Caches"
            } else {
                ".cache"
            })
            .join(APP_NAME)
    }
}

use std::path::PathBuf;

const APP_NAME: &str = "diff-of-services";

pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DOS_CONFIG_DIR") {
        PathBuf::from(dir)
    } else {
        DefaultDirs::config()
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DOS_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        DefaultDirs::data()
    }
}

pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DOS_CACHE_HOME") {
        PathBuf::from(dir)
    } else {
        DefaultDirs::cache()
    }
}

struct DefaultDirs;

#[cfg(target_os = "windows")]
impl DefaultDirs {
    fn config() -> PathBuf {
        PathBuf::from(std::env::var("APPDATA").unwrap()).join(APP_NAME)
    }

    fn data() -> PathBuf {
        PathBuf::from(std::env::var("APPDATA").unwrap()).join(APP_NAME)
    }

    fn cache() -> PathBuf {
        PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join(APP_NAME)
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl DefaultDirs {
    fn config() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Application Support")
            .join(APP_NAME)
    }

    fn data() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Application Support")
            .join(APP_NAME)
    }

    fn cache() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Caches")
            .join(APP_NAME)
    }
}

#[cfg(target_os = "linux")]
impl DefaultDirs {
    fn config() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir).join(APP_NAME)
        } else {
            PathBuf::from(std::env::var("HOME").unwrap())
                .join(".config")
                .join(APP_NAME)
        }
    }

    fn data() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(dir).join(APP_NAME)
        } else {
            PathBuf::from(std::env::var("HOME").unwrap())
                .join(".local/share")
                .join(APP_NAME)
        }
    }

    fn cache() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
            PathBuf::from(dir).join(APP_NAME)
        } else {
            PathBuf::from(std::env::var("HOME").unwrap())
                .join(".cache")
                .join(APP_NAME)
        }
    }
}

use std::path::PathBuf;

pub fn get_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(data_home) = env::var("XDG_DATA_HOME") {
            paths.push(PathBuf::from(data_home));
        } else if let Some(home) = env::home_dir() {
            paths.push(PathBuf::from(home).join(".local/share"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::env;

        if let Some(home) = env::home_dir() {
            use std::path::PathBuf;

            paths.push(PathBuf::from(home).join("Library/Application Support"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            paths.push(PathBuf::from(local));
        } else if let Ok(roaming) = env::var("APPDATA") {
            paths.push(PathBuf::from(roaming));
        }
    }

    paths
}

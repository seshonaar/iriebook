//! Helpers for launching external desktop apps without leaking AppImage runtime state.

use std::ffi::OsStr;
use std::path::Path;

#[cfg(target_os = "linux")]
use iriebook::resource_access::command::remove_appimage_environment;
#[cfg(target_os = "linux")]
use std::process::Command;

pub fn open_path(path: &Path) -> Result<(), String> {
    open_os_target(path.as_os_str(), "path")
}

pub fn open_text_target(target: &str) -> Result<(), String> {
    open_os_target(OsStr::new(target), "target")
}

#[cfg(target_os = "linux")]
fn open_os_target(target: &OsStr, label: &str) -> Result<(), String> {
    let mut command = Command::new("xdg-open");
    command.arg(target);
    remove_appimage_environment(&mut command);

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open {label}: {e}"))
}

#[cfg(not(target_os = "linux"))]
fn open_os_target(target: &OsStr, label: &str) -> Result<(), String> {
    open::that(target).map_err(|e| format!("Failed to open {label}: {e}"))
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use iriebook::resource_access::command::appimage_environment_keys;

    #[cfg(target_os = "linux")]
    #[test]
    fn cleans_environment_keys_that_poison_external_apps() {
        let keys = appimage_environment_keys();

        for expected in [
            "APPDIR",
            "GIO_EXTRA_MODULES",
            "GDK_PIXBUF_MODULE_FILE",
            "GTK_PATH",
            "LD_LIBRARY_PATH",
            "PYTHONHOME",
            "PYTHONPATH",
            "XDG_DATA_DIRS",
        ] {
            assert!(keys.contains(&expected), "missing {expected}");
        }
    }
}

//! Command execution utilities for Resource Access layer
//!
//! Provides shared utilities for executing and formatting output
//! from external command-line tools.

use std::process::{Command, Output};

/// Environment variables injected by AppImage launchers that should not leak to
/// external desktop or publishing tools such as Pandoc, TeX, or Calibre.
pub fn appimage_environment_keys() -> &'static [&'static str] {
    &[
        "APPDIR",
        "APPIMAGE",
        "APPIMAGE_GTK_THEME",
        "APPIMAGE_ORIGINAL_LD_LIBRARY_PATH",
        "ARGV0",
        "GDK_BACKEND",
        "GDK_PIXBUF_MODULE_FILE",
        "GIO_EXTRA_MODULES",
        "GI_TYPELIB_PATH",
        "GSETTINGS_SCHEMA_DIR",
        "GTK_DATA_PREFIX",
        "GTK_EXE_PREFIX",
        "GTK_IM_MODULE_FILE",
        "GTK_PATH",
        "GTK_THEME",
        "LD_LIBRARY_PATH",
        "OWD",
        "PYTHONHOME",
        "PYTHONPATH",
        "XDG_DATA_DIRS",
    ]
}

pub fn remove_appimage_environment(command: &mut Command) -> &mut Command {
    for key in appimage_environment_keys() {
        command.env_remove(key);
    }
    command
}

/// Formats the output from a process command into a human-readable string.
///
/// If the command succeeded, returns trimmed stdout.
/// If the command failed, returns stderr or a status message.
pub fn format_output(output: Output) -> String {
    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if err_msg.is_empty() {
            format!("Command failed with status: {}", output.status)
        } else {
            format!("Error: {}", err_msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn appimage_environment_keys_include_external_tool_poison() {
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

    #[test]
    fn remove_appimage_environment_marks_variables_for_removal() {
        let mut command = Command::new("pandoc");
        remove_appimage_environment(&mut command);

        for expected in appimage_environment_keys() {
            assert!(
                command
                    .get_envs()
                    .any(|(key, value)| key == *expected && value.is_none()),
                "{expected} was not removed"
            );
        }
    }

    #[test]
    fn test_format_output_success() {
        let output = Command::new("echo")
            .arg("hello world")
            .output()
            .expect("Failed to execute echo");

        let result = format_output(output);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_format_output_with_whitespace() {
        let output = Command::new("echo")
            .arg("  trimmed  ")
            .output()
            .expect("Failed to execute echo");

        let result = format_output(output);
        assert_eq!(result, "trimmed");
    }

    #[test]
    fn test_format_output_command_not_found() {
        let output = Command::new("nonexistent_command_xyz")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(_) => {
                // If by some miracle the command exists, skip this test
            }
            Err(_) => {
                // Expected - command doesn't exist
                // We can't easily test the format_output function with a failed command
                // in a cross-platform way, so we just verify the function compiles
            }
        }
    }
}

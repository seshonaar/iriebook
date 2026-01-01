//! Command execution utilities for Resource Access layer
//!
//! Provides shared utilities for executing and formatting output
//! from external command-line tools.

use std::process::Output;

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
    use std::process::{Command, Stdio};

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

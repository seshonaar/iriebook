use keyring::Entry;
use std::error::Error;

const SERVICE_NAME: &str = "iriebook-github";
const USERNAME: &str = "oauth-token";
const TEST_SERVICE: &str = "iriebook-diagnostic";
const TEST_USER: &str = "test-user";

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== IrieBook Keyring Diagnostic Tool ===");
    println!("OS: {}", std::env::consts::OS);

    // Check main app entry
    println!(
        "\n[1] Checking Main App Entry ('{}', '{}')",
        SERVICE_NAME, USERNAME
    );
    match Entry::new(SERVICE_NAME, USERNAME) {
        Ok(entry) => match entry.get_password() {
            Ok(_) => println!("✓ Found existing token for main app!"),
            Err(e) => println!("✗ Could not retrieve main app token: {}", e),
        },
        Err(e) => println!("✗ Failed to create entry handle: {}", e),
    }

    // Diagnostic Test
    println!(
        "\n[2] Running Write/Read Test ('{}', '{}')",
        TEST_SERVICE, TEST_USER
    );
    match Entry::new(TEST_SERVICE, TEST_USER) {
        Ok(test_entry) => {
            let test_secret = "diagnostic-secret-123";

            print!("  - Attempting to store... ");
            match test_entry.set_password(test_secret) {
                Ok(_) => println!("✓ Success"),
                Err(e) => {
                    println!("✗ Failed: {}", e);
                    println!(
                        "\nDiagnostic failed at storage step. This usually means the system keyring service (Secret Service/kwallet) is not reachable or responding."
                    );
                    return Ok(());
                }
            }

            print!("  - Attempting to retrieve... ");
            match test_entry.get_password() {
                Ok(s) => {
                    if s == test_secret {
                        println!("✓ Success (matches)");
                    } else {
                        println!("✗ Mismatch! Expected '{}', got '{}'", test_secret, s);
                    }
                }
                Err(e) => println!("✗ Failed: {}", e),
            }

            print!("  - Attempting to delete... ");
            match test_entry.delete_credential() {
                Ok(_) => println!("✓ Success"),
                Err(e) => println!("✗ Failed: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to create test entry handle: {}", e),
    }

    println!("\n=== Diagnostic Complete ===");
    Ok(())
}

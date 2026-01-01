use std::fs;
use std::path::Path;
use base64::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <credentials_dir>", args[0]);
        std::process::exit(1);
    }

    let creds_dir = Path::new(&args[1]);
    if !creds_dir.is_dir() {
        eprintln!("Error: {} is not a directory", args[1]);
        std::process::exit(1);
    }

    let google_json = creds_dir.join("google.json");
    let github_json = creds_dir.join("github.json");

    if google_json.exists() {
        println!("Found google.json, embedding...");
        embed_creds(&google_json, "iriebook/src/resource_access/google_auth.rs", "GOOGLE_CREDENTIALS_B64")?;
    } else {
        println!("google.json not found in {}", args[1]);
    }

    if github_json.exists() {
        println!("Found github.json, embedding...");
        embed_creds(&github_json, "iriebook/src/resource_access/github_auth.rs", "GITHUB_CREDENTIALS_B64")?;
    } else {
        println!("github.json not found in {}", args[1]);
    }

    Ok(())
}

fn embed_creds(json_path: &Path, rs_path: &str, const_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json_content = fs::read(json_path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(json_content);

    let rs_content = fs::read_to_string(rs_path)?;
    
    let replacement = format!("const {}: &str = \"{}\";", const_name, b64);
    
    // We expect the pattern to exist. If not, we might want to be more robust, 
    // but for this internal tool it should be fine.
    if !rs_content.contains(&format!("const {}: &str =", const_name)) {
        return Err(format!("Could not find constant {} in {}", const_name, rs_path).into());
    }

    // Find the line and replace it
    let mut lines: Vec<String> = rs_content.lines().map(|s| s.to_string()).collect();
    let mut found = false;
    for line in lines.iter_mut() {
        if line.trim().starts_with(&format!("const {}: &str =", const_name)) {
            *line = replacement.clone();
            found = true;
            break;
        }
    }

    if !found {
         return Err(format!("Could not find constant {} in {} (line matching failed)", const_name, rs_path).into());
    }

    let new_content = lines.join("\n") + "\n";
    fs::write(rs_path, new_content)?;

    println!("Successfully embedded credentials into {}", rs_path);
    Ok(())
}

use regex::Regex;
use std::env;
use std::fs;

#[derive(Debug)]
enum ProjectType {
    Vue,  // Frontend (package.json found)
    Rust, // Backend (Cargo.toml found)
}

fn detect_project_type() -> Option<ProjectType> {
    let cwd = env::current_dir().ok()?;

    // Check for package.json (Vue frontend)
    if cwd.join("package.json").exists() {
        return Some(ProjectType::Vue);
    }

    // Check for Cargo.toml (Rust backend)
    if cwd.join("Cargo.toml").exists() {
        return Some(ProjectType::Rust);
    }

    None
}

fn find_version_file(project_type: &ProjectType) -> Option<String> {
    let cwd = env::current_dir().ok()?;

    match project_type {
        ProjectType::Vue => {
            let vue_path = cwd.join("src").join("App.vue");
            if vue_path.exists() {
                Some(vue_path.to_string_lossy().to_string())
            } else {
                None
            }
        }
        ProjectType::Rust => {
            // Try src/handlers/versions.rs first, then version.rs
            let versions_rs = cwd.join("src").join("handlers").join("versions.rs");
            let version_rs = cwd.join("src").join("handlers").join("version.rs");

            if versions_rs.exists() {
                Some(versions_rs.to_string_lossy().to_string())
            } else if version_rs.exists() {
                Some(version_rs.to_string_lossy().to_string())
            } else {
                None
            }
        }
    }
}

type ContentAndVersion = (String, String);

fn parse_and_increment_version(data: &str, part: &str) -> Option<ContentAndVersion> {
    // More flexible patterns - allow optional type annotation (but not too greedy)
    // Try double quotes first - use case-insensitive flag to match VERSION/version
    // Capture: group 1 = var name, group 2 = type annotation, group 3 = spaces before =, group 4 = version
    let re_double = Regex::new("(?i)const\\s+(version)(\\s*:\\s*[^=]+)?(\\s*)=\\s*\"([\\d.]+)\"").ok()?;
    // Then pattern for single quotes  
    // Fix: preserve whitespace around = by capturing group 3 (before =) and adding one space after
    let re_single = Regex::new("(?i)const\\s+(version)(\\s*:\\s*[^=]+)?(\\s*)=\\s+'([\\d.]+)'").ok()?;

    // Try double quotes first
    if let Some(caps) = re_double.captures(data) {
        let var_name = caps.get(1)?.as_str();
        let type_annotation = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let spaces_before_eq = caps.get(3).unwrap().as_str();
        let version_str = caps.get(4)?.as_str();
        
        let mut parts: Vec<u32> = version_str
            .split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        if parts.len() != 3 {
            return None;
        }

        match part {
            "major" => { parts[0] += 1; parts[1] = 0; parts[2] = 0; }
            "minor" => { parts[1] += 1; parts[2] = 0; }
            "build" | _ => { parts[2] += 1; }
        }

        let new_version = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
        
        // Replace preserving variable name, type annotation, and original whitespace before =
        let full_match = caps.get(0).unwrap().as_str();
        let replacement = format!("const {}{}{}= \"{}\"", var_name, type_annotation, spaces_before_eq, new_version);
        return Some((data.replace(full_match, &replacement), new_version));
    }

    // Try single quotes
    if let Some(caps) = re_single.captures(data) {
        let var_name = caps.get(1)?.as_str();
        let type_annotation = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let spaces_before_eq = caps.get(3).unwrap().as_str();
        let version_str = caps.get(4)?.as_str();
        
        let mut parts: Vec<u32> = version_str
            .split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        if parts.len() != 3 {
            return None;
        }

        match part {
            "major" => { parts[0] += 1; parts[1] = 0; parts[2] = 0; }
            "minor" => { parts[1] += 1; parts[2] = 0; }
            "build" | _ => { parts[2] += 1; }
        }

        let new_version = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
        
        // Replace preserving variable name, type annotation, and single quotes
        let full_match = caps.get(0).unwrap().as_str();
        let replacement = format!("const {}{}{}= '{}'", var_name, type_annotation, spaces_before_eq, new_version);
        return Some((data.replace(full_match, &replacement), new_version));
    }

    None
}

fn main() {
    // Get the part to increment from command line argument
    // Default is "build" if no argument provided
    let args: Vec<String> = env::args().collect();
    
    let part = if args.len() > 1 {
        let arg = &args[1];
        if arg == "major" || arg == "minor" || arg == "build" {
            arg.as_str()
        } else {
            eprintln!("Error: Invalid argument '{}'. Use 'major', 'minor', or 'build'.", arg);
            std::process::exit(1);
        }
    } else {
        "build"
    };

    // Detect project type
    let project_type = match detect_project_type() {
        Some(pt) => pt,
        None => {
            eprintln!("Error: Cannot detect project type. Neither package.json nor Cargo.toml found in current directory.");
            std::process::exit(1);
        }
    };

    // Find the version file
    let file_path = match find_version_file(&project_type) {
        Some(path) => path,
        None => {
            match project_type {
                ProjectType::Vue => {
                    eprintln!("Error: App.vue not found in src/ directory.");
                }
                ProjectType::Rust => {
                    eprintln!("Error: version.rs or versions.rs not found in src/handlers/ directory.");
                }
            }
            std::process::exit(1);
        }
    };

    // Read the file
    let data = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error: Could not read file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    // Parse and increment version
    let (updated_data, version) = match parse_and_increment_version(&data, part) {
        Some(new_content) => new_content,
        None => {
            eprintln!("Error: Could not find version string in the expected format (const version = \"X.Y.Z\" or const version = 'X.Y.Z').");
            std::process::exit(1);
        }
    };

    // Write back to file
    if let Err(e) = fs::write(&file_path, &updated_data) {
        eprintln!("Error: Could not write to file '{}': {}", file_path, e);
        std::process::exit(1);
    }

    println!("✨Updated '{}' to version {}", file_path, version);
}
use std::fs;
use std::process::Command;
use serial_test::serial;

const UPVER_EXE: &str = "D:\\Winnie\\icci\\apps\\rust_libs\\tools\\upver\\target\\debug\\upver.exe";

fn get_version_from_file(path: &str) -> String {
    let content = fs::read_to_string(path).expect(&format!("Failed to read file: {}", path));
    
    // Extract version - handle type annotations like : &'static str
    // Try double quotes with optional type annotation
    let re_double = regex::Regex::new(r#"(?i)const\s+version\s*(\s*:\s*[^=]+)?\s*=\s*"([\d.]+)""#).unwrap();
    // Then single quotes with optional type annotation
    let re_single = regex::Regex::new(r#"(?i)const\s+version\s*(\s*:\s*[^=]+)?\s*=\s*'([\d.]+)'"#).unwrap();
    
    if let Some(caps) = re_double.captures(&content) {
        caps.get(2).unwrap().as_str().to_string()
    } else if let Some(caps) = re_single.captures(&content) {
        caps.get(2).unwrap().as_str().to_string()
    } else {
        panic!("No version found in: {}", path);
    }
}

fn set_version(path: &str, version: &str) {
    let content = fs::read_to_string(path).unwrap();
    
    // Replace version - handle both patterns and preserve type annotation
    // Pattern for double quotes: const VERSION: 'static str = "X.Y.Z" -> const VERSION: 'static str = "new"
    // Pattern for single quotes: const version = 'X.Y.Z' -> const version = 'new'
    // Updated to capture: group 1 = const version, group 2 = type annotation, group 3 = whitespace before =
    let re_double = regex::Regex::new(r#"(?i)(const\s+version)(\s*:\s*[^=]+)?(\s*)=\s*"[\d.]+""#).unwrap();
    let re_single = regex::Regex::new(r#"(?i)(const\s+version)(\s*:\s*[^=]+)?(\s*)=\s*'[\d.]+'"#).unwrap();
    
    let new_content = if re_double.is_match(&content) {
        re_double.replace(&content, |caps: &regex::Captures| {
            let prefix = caps.get(1).unwrap().as_str();
            let type_annot = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let spaces_before_eq = caps.get(3).unwrap().as_str();
            format!("{}{}{}= \"{}\"", prefix, type_annot, spaces_before_eq, version)
        }).to_string()
    } else if re_single.is_match(&content) {
        re_single.replace(&content, |caps: &regex::Captures| {
            let prefix = caps.get(1).unwrap().as_str();
            let type_annot = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let spaces_before_eq = caps.get(3).unwrap().as_str();
            format!("{}{}{}= '{}'", prefix, type_annot, spaces_before_eq, version)
        }).to_string()
    } else {
        panic!("Cannot find version line in: {}", path);
    };
    
    fs::write(path, new_content).expect("Failed to write file");
}

fn run_upver(part: &str, cwd: &str) {
    let output = Command::new(UPVER_EXE)
        .arg(part)
        .current_dir(cwd)
        .output()
        .expect("Failed to run upver");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("upver failed: {}", stderr);
    }
}

fn run_upver_no_arg(cwd: &str) {
    let output = Command::new(UPVER_EXE)
        .current_dir(cwd)
        .output()
        .expect("Failed to run upver");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("upver failed: {}", stderr);
    }
}

const RUST_VERSION_FILE: &str = "D:\\Winnie\\icci\\apps\\rust_libs\\tools\\upver\\tests\\rust\\src\\handlers\\versions.rs";
const VUE_VERSION_FILE: &str = "D:\\Winnie\\icci\\apps\\rust_libs\\tools\\upver\\tests\\vue\\src\\App.vue";
const RUST_DIR: &str = "D:\\Winnie\\icci\\apps\\rust_libs\\tools\\upver\\tests\\rust";
const VUE_DIR: &str = "D:\\Winnie\\icci\\apps\\rust_libs\\tools\\upver\\tests\\vue";

// ==================== Rust Tests ====================

#[test]
#[serial]
fn test_rust_major_increment() {
    // Reset to known state
    set_version(RUST_VERSION_FILE, "1.2.3");
    
    run_upver("major", RUST_DIR);
    
    let new_version = get_version_from_file(RUST_VERSION_FILE);
    assert_eq!(new_version, "2.0.0", "Major increment should reset minor and build");
}

#[test]
#[serial]
fn test_rust_minor_increment() {
    // Reset to known state
    set_version(RUST_VERSION_FILE, "1.2.3");
    
    run_upver("minor", RUST_DIR);
    
    let new_version = get_version_from_file(RUST_VERSION_FILE);
    assert_eq!(new_version, "1.3.0", "Minor increment should reset build");
}

#[test]
#[serial]
fn test_rust_build_increment() {
    // Reset to known state
    set_version(RUST_VERSION_FILE, "1.2.3");
    
    run_upver("build", RUST_DIR);
    
    let new_version = get_version_from_file(RUST_VERSION_FILE);
    assert_eq!(new_version, "1.2.4", "Build increment should just add 1");
}

// ==================== Vue Tests ====================

#[test]
#[serial]
fn test_vue_major_increment() {
    // Reset to known state
    set_version(VUE_VERSION_FILE, "2.5.7");
    
    run_upver("major", VUE_DIR);
    
    let new_version = get_version_from_file(VUE_VERSION_FILE);
    assert_eq!(new_version, "3.0.0", "Major increment should reset minor and build");
}

#[test]
#[serial]
fn test_vue_minor_increment() {
    // Reset to known state
    set_version(VUE_VERSION_FILE, "2.5.7");
    
    run_upver("minor", VUE_DIR);
    
    let new_version = get_version_from_file(VUE_VERSION_FILE);
    assert_eq!(new_version, "2.6.0", "Minor increment should reset build");
}

#[test]
#[serial]
fn test_vue_build_increment() {
    // Reset to known state
    set_version(VUE_VERSION_FILE, "2.5.7");
    
    run_upver("build", VUE_DIR);
    
    let new_version = get_version_from_file(VUE_VERSION_FILE);
    assert_eq!(new_version, "2.5.8", "Build increment should just add 1");
}

// ==================== Default (no argument) Tests ====================

#[test]
#[serial]
fn test_rust_default_increment() {
    // Reset to known state
    set_version(RUST_VERSION_FILE, "0.9.9");
    
    // Run without argument (should default to build)
    run_upver_no_arg(RUST_DIR);
    
    let new_version = get_version_from_file(RUST_VERSION_FILE);
    assert_eq!(new_version, "0.9.10", "Default should increment build");
}

#[test]
#[serial]
fn test_vue_default_increment() {
    // Reset to known state
    set_version(VUE_VERSION_FILE, "1.0.0");
    
    // Run without argument (should default to build)
    run_upver_no_arg(VUE_DIR);
    
    let new_version = get_version_from_file(VUE_VERSION_FILE);
    assert_eq!(new_version, "1.0.1", "Default should increment build");
}
use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::from_path;
use std::env;
use std::path::PathBuf;
use std::process::Command;

// ANSI Color Codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";

fn print_success(msg: &str) {
    println!("{}✅ {} {}{}", GREEN, BOLD, msg, RESET);
}

fn print_error(msg: &str) {
    eprintln!("{}❌ {} {}{}", RED, BOLD, msg, RESET);
}

fn print_step(msg: &str) {
    println!("{}🚀 {} {}{}", CYAN, BOLD, msg, RESET);
}

fn print_detail(msg: &str) {
    println!("{}🔍 {} {}{}", YELLOW, msg, "", RESET);
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Syncs a folder to S3 and invalidates CloudFront", long_about = None)]
struct Args {
    /// The folder containing the .env file and files to sync
    folder: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let folder = args.folder;

    if !folder.is_dir() {
        print_error(&format!("Provided path is not a directory: {:?}", folder));
        std::process::exit(1);
    }

    let env_path = folder.join(".env");
    if !env_path.exists() {
        print_error(&format!(".env file not found in {:?}", folder));
        std::process::exit(1);
    }

    // Load .env into the environment
    from_path(&env_path).context("Failed to load .env file")?;

    let s3_bucket = env::var("S3BUCKET").context("S3BUCKET not found in .env")?;
    let cf_id = env::var("CFID").context("CFID not found in .env")?;
    let aws_profile = env::var("AWS_PROFILE").ok();

    println!("{}✨ {} {}{}", BLUE, BOLD, "Starting Deployment Pipeline", RESET);
    println!("{}", "=".repeat(40));
    print_detail(&format!("Target S3 Bucket: {}", s3_bucket));
    print_detail(&format!("Target CloudFront ID: {}", cf_id));
    if let Some(ref profile) = aws_profile {
        print_detail(&format!("AWS Profile: {}", profile));
    }
    print_detail(&format!("Working directory: {:?}", folder));
    println!("{}", "=".repeat(40));

    // 1. AWS S3 Sync
    print_step("Running S3 sync...");
    let mut s3_cmd = Command::new("aws");
    s3_cmd.arg("s3").arg("sync").arg(".").arg(format!("s3://{}/", s3_bucket))
        .arg("--delete")
        .arg("--exclude").arg(".git*")
        .arg("--exclude").arg(".gitignore")
        .arg("--exclude").arg(".claude*")
        .arg("--exclude").arg(".playwright-mcp*")
        .arg("--exclude").arg(".sisyphus*")
        .arg("--exclude").arg("qa-screenshots*")
        .arg("--exclude").arg("scripts*")
        .arg("--exclude").arg("data*")
        .arg("--exclude").arg(".env")
        .arg("--exclude").arg(".opencode*")
        .current_dir(&folder);

    if let Some(ref profile) = aws_profile {
        s3_cmd.arg("--profile").arg(profile);
    }

    let s3_status = s3_cmd.status().context("Failed to execute aws s3 sync")?;

    if !s3_status.success() {
        print_error(&format!("S3 sync failed with exit code: {}", s3_status));
        std::process::exit(1);
    }
    print_success("S3 sync completed successfully.");

    // 2. AWS CloudFront Invalidation
    print_step("Running CloudFront invalidation...");
    let mut cf_cmd = Command::new("aws");
    cf_cmd.arg("cloudfront").arg("create-invalidation")
        .arg("--distribution-id").arg(&cf_id)
        .arg("--paths").arg("/*")
        .current_dir(&folder);

    if let Some(ref profile) = aws_profile {
        cf_cmd.arg("--profile").arg(profile);
    }

    let cf_status = cf_cmd.status().context("Failed to execute aws cloudfront create-invalidation")?;

    if !cf_status.success() {
        print_error(&format!("CloudFront invalidation failed with exit code: {}", cf_status));
        std::process::exit(1);
    }
    print_success("CloudFront invalidation completed successfully.");
    
    println!("{}", "=".repeat(40));
    print_success("🚀 Deployment finished!");

    Ok(())
}

# Code generator for iciaws_router

This is a small tool to help generate the routes.rs for iciaws_router based route handlers, as well as temple.yaml for local testing.

## Build the tool

```sh
cargo build --release
cargo install
```

## Usage

Run this code in a parent folder, and specify a Rust project folder where Cargo.toml resides.

```sh
gen_router <command> <project_path>
```

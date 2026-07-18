# upver - Update the version number of a specified codebase.

- It handles Vue-based frontend apps where the version string is in App.vue.
- It can also handle Rust-based backend services where the version string is in the handlers/versions.rs

## Requirements

- Determine the source language by checking the existence of package.json and Cargo.toml under the project folder (current folder).
    - If package.json found, it's a Vue-based frontend
    - If Cargo.toml found, it's a Rust-based backend
- Find the file to update: 
    - if it's a frontend, get file App.vue in src folder under the project folder (current folder);
    - if it's a backend, get file versions.rs or version.rs in src/handlers folder under the project folder
- Increment the version:
    - search the file to find a const version string in the form of '<major>.<minor>.<build>' (3 parts separated by dots)
    - parse current const version string into three numbers, representing the major, minor, and build parts
    - increment the build by default, if specified on the command line argument minor or major, increment the number accordingly
    - remember to reset the lesser version numbers, ie, if major incremented, minor and build should be 0, if minor added, build should be 0
    - save back the source file after update
- Command line argument:
    - Run upver without an argument will increment the build part of the version
    - Run upver major|minor will increment the part accordingly
- Error message:
    - if neither App.vue nor versions rs file found, or const version not found, show error and quit

## AI-generated code

### Features implemented:
- Detects project type by checking for package.json (Vue) or Cargo.toml (Rust)
- Finds version file: src/App.vue for Vue, src/handlers/versions.rs or version.rs for Rust
- Parses and increments version in X.Y.Z format
- Command line arguments:
  - No argument → increments build (default)
  - major → increments major, resets minor and build to 0
  - minor → increments minor, resets build to 0  
  - build → increments build only
- Preserves original whitespace and formatting in Rust files (including type annotations like &'static str)
- Handles both single quotes and double quotes

### Project structure:
- src/main.rs - Main implementation (194 lines)
- tests/integration_test.rs - Integration tests
- tests/rust/ and tests/vue/ - Test fixtures

### Build:

```sh
cargo build
```

### Usage:

```sh
upver          # Increment build (default)
upver major    # Increment major
upver minor    # Increment minor  
upver build    # Increment build
```

## Manual test

- Open versions.rs file and check the changes

```sh
cd tests/rust
../../target/debug/upver.exe major
```

## Build release and install

```sh
cargo build --release
cargo install .
```

## Integrated Test (generated)

cargo test

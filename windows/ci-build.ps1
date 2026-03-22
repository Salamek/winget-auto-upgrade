# Prepend Cargo to PATH
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

# Build Rust project
cargo build --release

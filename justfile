# Default recipe
default:
    @just --list

# Run the Dioxus dashboard UI with a clean environment and Doppler secrets
run-ui:
    doppler run -- env -u LD_LIBRARY_PATH -u GTK_PATH cargo run -p dashboard

# Build the entire workspace
build:
    cargo build

# Check the entire workspace
check:
    cargo check

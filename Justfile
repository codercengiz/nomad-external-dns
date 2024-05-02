# Justfile

set dotenv-load

test:
    echo "Running tests..."
    cargo test -- --nocapture
    find . -name "temp_nomad_job*" -type f -exec rm {} +
    echo "Tests passed!"


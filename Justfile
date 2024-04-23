# Justfile

set dotenv-load

test:
    echo "Building the application..."
    cargo build --release

    echo "Building Docker images..."
    docker build -t nomad-external-dns:local . -f Containerfile.test

    echo "Running tests..."
    cargo test

    #echo "Cleaning up..."
    just cleanup

cleanup:
    @echo "Stopping and removing docker containers..."
    docker stop consul-dev || true

    @echo "Removing Docker images..."
    docker rmi nomad-external-dns:local || true

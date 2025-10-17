GROUP: 42
MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

# Server

A spec-compliant server that connects to other servers.

The communications between this server and the client is custom, and is not part of the specification.

## Development

```bash
cd server
cargo build
cargo run
```

## Linting & Formatting

```bash
# Check for linting issues
cargo clippy --all-targets --all-features -- -D warnings

# Check for formatting issues
cargo fmt --check

# Fix formatting issues
cargo fmt
```

## Testing

```bash
# Run all tests
cargo test

# Run a specific test
cargo test name_of_test
```

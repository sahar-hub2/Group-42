GROUP: 42
MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

# Client

## Development


```bash
# Install dependencies
cd client
bun install
```

```bash
# Run Tauri desktop dev
bun run dev:desktop

# Run client in browser
bun run dev:web
```

## Linting & Formatting
```bash
# Check for linting issues
bun lint

# Fix auto-fixable linting issues
bun lint --fix

# Check for formatting issues
bun format:check

# Fix formatting issues
bun format
```

## Testing
```bash
# Run all tests
bun test

# Run a specific test
bun test ./tests/specific-file.test.ts
```
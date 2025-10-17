GROUP: 42
MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

# Testing

This directory contains scripts and configuration files for testing the secure chat protocol (servers and clients) holistically.

## Setup

### Automatic Setup (Recommended)

> [!NOTE]
> If you are on macOS, you will need to run with sudo for the script to work properly.
> This is due to server and client processes being managed by the testing script.

The Testing CLI (which invokes the Python CLI) will automatically set up its dependencies when first run:

```bash
./test-cli
```

This will:

1. Create a Python virtual environment in `.venv/`
2. Install required Python dependencies
3. Show the default help menu

### Manual Setup

If you prefer to set up the environment manually:

```bash
# Set up venv and install dependencies
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# Run the CLI directly with Python
python3 test_cli.py
```

## Usage

### Quick Start

```bash
# Show current status
./test-cli status

# Build the server
./test-cli build

# Start all servers with 3 second delays
./test-cli start-all --delay 3

# Stop all servers
./test-cli stop-all

# Run a quick bootstrap demonstration
./test-cli demo
```

### Individual Server Management

```bash
# Start server 1 in foreground (see live output)
./test-cli start 1

# Start server 2 in background
./test-cli start 2 --background

# Stop server 3
./test-cli stop 3
```

### Log Monitoring

```bash
# Show last 20 lines of server 1 logs
./test-cli logs 1

# Show last 50 lines of server 2 logs  
./test-cli logs 2 --lines 50

# Follow server 3 logs in real-time (like tail -f)
./test-cli logs 3 --follow
```

### Key Generation and Utilities

```bash
# Generate new RSA keys for all servers
./test-cli generate-keys

# Get help for any command
./test-cli --help
./test-cli start --help
./test-cli logs --help
```

### Manual Testing

```bash
# Terminal 1: Start the bootstrap server
./test-cli start 1

# Terminal 2: Start a client server (wait for Server 1 to be ready)
./test-cli start 2

# Terminal 3: Start another client server (wait for both to be ready)
./test-cli start 3
```

### Monitoring

- Logs are saved to the `logs` folder
- PIDs are tracked in `logs/server*.pid` files. If you delete these, the `stop-all` command will not work properly
- Use `./test-cli logs 1 --follow` to monitor server output in real-time

## Testing Scenarios

### Basic Bootstrap Test

1. Start Server 1 (standalone)
2. Start Server 2 (should connect to Server 1)
3. Check logs for successful bootstrap messages

### Multi-Server Bootstrap Test  

1. Start Server 1 (standalone)
2. Start Server 2 (connects to Server 1)
3. Start Server 3 (connects to both Server 1 and Server 2)
4. Verify Server 3 successfully bootstraps from available servers

### Error Handling Test

1. Start Server 2 without Server 1 running
2. Observe bootstrap failure and retry behavior
3. Start Server 1 and see if Server 2 eventually connects

## Regenerating Keys

If you need to regenerate the test keys:

```bash
./test-cli generate-keys
```

This will create new private/public key pairs for all three servers and update the configuration files accordingly.

## Environment Variables

The servers use these environment variables:

- `CONFIG_FILE` - Path to configuration file
- `PRIVATE_KEY_FILE` - Path to private key file
- `HOST` - Server host (default: 127.0.0.1)
- `PORT` - Server port (default per server)

## Network Ports

- Server 1: 3001
- Server 2: 3002
- Server 3: 3003

Make sure these ports are available before running the tests.

## Troubleshooting

### Port Already in Use

```bash
# Check what's using the ports
lsof -i :3001
lsof -i :3002  
lsof -i :3003

# Stop all test servers
./test-cli stop-all
```

### Build Errors

```bash
# Build the server using CLI
./test-cli build

# Or manually build the server
cd ../server
cargo build
```

### Permission Errors

```bash
# Make CLI scripts executable
chmod +x servers
chmod +x test_cli.py
```

### Python Dependencies

```bash
# If dependencies fail to install
pip install --upgrade pip
pip install -r requirements.txt

# Or let the CLI handle it automatically
./test-cli status  # Will install dependencies if needed
```

### Virtual Environment Issues

```bash
# Remove and recreate virtual environment
rm -rf .venv
./test-cli status  # Will recreate automatically
```

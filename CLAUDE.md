# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a cross-platform NAT traversal software written in Rust, consisting of a server and client architecture. The server runs on a public server to facilitate connections, while the client runs on local machines behind NAT to expose local services.

## Architecture

The project is organized as a Cargo workspace with the following components:

- **common/**: Shared protocol definitions, configuration, and utilities
- **server/**: Server-side application that manages client connections and tunnels
- **client/**: Client-side application with both CLI and GUI interfaces
- **platform/**: Platform-specific integrations (Windows services, Linux systemd)

## Key Technologies

- **Async Runtime**: tokio for all async operations
- **TLS Security**: rustls for encrypted communications
- **GUI Framework**: egui for cross-platform client interface
- **Serialization**: serde with JSON for protocol messages, TOML for configuration
- **Logging**: tracing with structured logging

## Build and Development Commands

### Build Commands
```bash
# Build entire workspace (Linux)
cargo build

# Build without GUI (avoids GTK dependencies)
cargo build -p nat-traversal-client --no-default-features

# Build specific components
cargo build -p nat-traversal-server
cargo build -p nat-traversal-client
cargo build -p nat-traversal-common
cargo build -p nat-traversal-platform

# Build for release
cargo build --release

# Cross-compile for Windows (requires mingw toolchain)
cargo build --target x86_64-pc-windows-gnu --release

# Build Windows GUI client specifically
cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release
```

### Cross-Platform Build Requirements

#### Linux Build Requirements
```bash
# For GUI client (egui/GTK dependencies)
sudo apt update
sudo apt install -y libgtk-3-dev libatk1.0-dev libcairo-gobject2 \
  libcairo2-dev libgdk-pixbuf2.0-dev libgio2.0-cil-dev \
  libglib2.0-dev libpango1.0-dev pkg-config

# For cross-compilation to Windows
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

#### Windows Build Requirements
```powershell
# For GUI client, no additional system dependencies needed
# rustls and egui are pure Rust implementations
cargo build --release
```

### Feature Flags
```bash
# Client with GUI (default)
cargo build -p nat-traversal-client

# Client without GUI (CLI only)
cargo build -p nat-traversal-client --no-default-features

# Available features:
# - gui: Enables egui-based graphical interface (default)
```

### Running Commands
```bash
# Run server (requires TLS certificates)
cargo run --bin nat-server

# Generate server config
cargo run --bin nat-server -- --generate-config

# Run client with GUI
cargo run --bin nat-client

# Run client in CLI mode
cargo run --bin nat-client -- --no-gui

# Generate client config
cargo run --bin nat-client -- --generate-config
```

### Testing
```bash
# Run all tests (excludes GUI components on systems without GTK)
cargo test -p nat-traversal-common -p nat-traversal-server -p nat-traversal-platform

# Test client without GUI
cargo test -p nat-traversal-client --no-default-features

# Test specific crate
cargo test -p nat-traversal-common
```

## Compilation Status and Known Issues

### ✅ Successfully Compiling Components
- `nat-traversal-common`: Core protocol and utilities
- `nat-traversal-server`: Server binary with TLS support  
- `nat-traversal-platform`: Cross-platform service integration
- `nat-traversal-client`: CLI client (without GUI)

### ⚠️ Platform-Specific Requirements
- **Linux GUI**: Requires GTK development libraries
- **Windows Cross-compilation**: Requires mingw-w64 toolchain
- **TLS Support**: Uses rustls (pure Rust, no OpenSSL dependency)

### 🔧 Fixed Compilation Issues
1. **Dependencies**: Added missing `tracing-appender`, `webpki-roots`, `hex`, `libc`
2. **TLS Types**: Fixed `tokio_rustls::TlsStream` type mismatches between client/server
3. **API Updates**: Updated deprecated `add_server_trust_anchors` to `add_trust_anchors`
4. **Conditional Compilation**: Added `#[cfg(feature = "gui")]` for optional GUI features
5. **Import Cleanup**: Removed unused imports and fixed module dependencies

### 📦 Binary Outputs
```bash
# Linux binaries
./target/release/nat-server          # Server executable
./target/release/nat-client          # Client executable (CLI mode)

# Windows binaries (after cross-compilation)
./target/x86_64-pc-windows-gnu/release/nat-server.exe
./target/x86_64-pc-windows-gnu/release/nat-client.exe
```

## Configuration

### Server Configuration (server.toml)
- Network binding and port settings
- TLS certificate paths
- Authentication tokens
- Rate limiting and connection limits

### Client Configuration (client.toml)
- Server connection details
- Tunnel configurations
- GUI preferences
- Logging settings

## Protocol Architecture

The system uses a custom binary protocol over TLS:

1. **Authentication**: Token-based auth with client IDs
2. **Tunnel Management**: Create/close tunnels with port mapping
3. **Data Forwarding**: Bidirectional data transfer through established tunnels
4. **Health Monitoring**: Ping/pong heartbeat and status reporting

## Cross-Platform Considerations

- **TLS**: Uses rustls (pure Rust) to avoid OpenSSL dependencies
- **GUI**: egui provides native look and feel on Windows/Linux
- **Services**: Platform-specific service integration via platform/ crate
- **Configuration**: Uses directories crate for platform-appropriate config paths

## Key Design Patterns

- **Async Message Passing**: All components communicate via mpsc channels
- **Shared State**: Arc<RwLock<T>> for thread-safe shared data
- **Error Handling**: Custom error types with anyhow for error propagation
- **Modular Design**: Clear separation between network, GUI, and business logic

## Development Notes

### Build Status Summary
- ✅ **Linux Native**: All components compile successfully
- ✅ **Tests**: All unit tests pass (3 crypto tests in common crate)
- ✅ **CLI Client**: Works without GUI dependencies
- ✅ **GUI Client (Linux)**: Requires GTK system libraries
- ✅ **Windows Cross-compilation**: Successfully generates Windows binaries
- ✅ **Windows GUI Client**: Full GUI support with egui framework

### Successful Windows Compilation
The project successfully cross-compiles to Windows with full GUI support:

**Generated Windows Binaries:**
```bash
./target/x86_64-pc-windows-gnu/release/nat-client.exe    # ~20MB - GUI Client
./target/x86_64-pc-windows-gnu/release/nat-server.exe    # ~11MB - Server
```

**Windows Compilation Requirements:**
- `gcc-mingw-w64-x86-64` toolchain
- `x86_64-pc-windows-gnu` Rust target
- All dependencies are pure Rust (no external DLLs needed)

**Fixed Windows-specific Issues:**
1. **egui API Compatibility**: Updated from `ViewportBuilder` to `initial_window_size`
2. **winapi Features**: Added explicit winapi dependency with required features
3. **Borrowing Issues**: Resolved mutable borrow conflicts in GUI code
4. **TLS Integration**: Full rustls support without OpenSSL dependencies

### TLS Certificate Requirements
- The server requires TLS certificates for production use
- For development, certificates can be self-signed:
```bash
# Generate self-signed certificate for development
openssl req -x509 -newkey rsa:4096 -keyout server.key -out server.crt -days 365 -nodes
```

### Feature Flag Architecture
The client supports conditional compilation:
- **Default**: Includes GUI support (requires system dependencies)
- **CLI-only**: `--no-default-features` for headless environments
- **GUI**: `--features gui` to explicitly enable GUI components

### Common Development Issues
1. **GTK Missing**: Use `--no-default-features` for client or install GTK dev libraries
2. **Windows Cross-compilation**: Install `gcc-mingw-w64-x86-64` package
3. **TLS Errors**: Ensure certificates are properly configured in config files
4. **Config Generation**: Check `~/.config/nat-traversal/` for generated config files

### Code Quality
- All components compile with only minor warnings (unused code)
- Memory safety guaranteed by Rust's ownership system
- Async architecture throughout for high performance
- Error handling via custom `NatError` type with anyhow integration

## Security Features

- Mandatory TLS encryption for all communications
- Token-based authentication
- Port range restrictions for tunnels
- Connection rate limiting
- Certificate verification (configurable)
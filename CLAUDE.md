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
The server requires TLS certificates for all connections. Here are the setup methods:

#### Development Certificate (Self-Signed)
```bash
# Method 1: Simple self-signed certificate
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=NAT-Traversal/CN=localhost"

# Method 2: Certificate with SAN (Subject Alternative Names)
# Create config file first:
cat > server.conf << EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = State  
L = City
O = NAT-Traversal
CN = localhost

[v3_req]
basicConstraints = CA:FALSE
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = YOUR_SERVER_IP
IP.1 = 127.0.0.1
IP.2 = YOUR_SERVER_IP
EOF

# Generate certificate with SAN
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server.conf
```

#### Production Certificate
For production environments, use certificates from a trusted CA like Let's Encrypt.

#### TLS Verification Settings
- **Development**: Set `tls_verify = false` in client config for self-signed certificates
- **Production**: Set `tls_verify = true` and use proper CA-signed certificates

**Important**: The client requires the `dangerous_configuration` feature for rustls to disable certificate verification in development mode.

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

## WSL + Windows 部署指南

这是一个常见的开发场景：在WSL中运行server，在Windows宿主机中运行client。

### 环境准备

#### WSL 端 (Server)
```bash
# 安装必要的构建工具
sudo apt update
sudo apt install -y gcc-mingw-w64-x86-64 build-essential pkg-config

# 添加 Windows 编译目标
rustup target add x86_64-pc-windows-gnu

# 获取 WSL IP 地址
ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1
# 示例输出: 172.22.247.72
```

#### Windows 端 (Client)
无需额外安装，编译后的 exe 文件可直接运行。

### 部署步骤

#### 1. 构建 Server 端
```bash
# 生成 server 配置
cargo run --bin nat-server -- --generate-config

# 构建 server (release 模式推荐)
cargo build --bin nat-server --release
```

#### 2. 配置 Server
编辑 `~/.config/nat-traversal/server.toml`:
```toml
[network]
bind_addr = "WSL_IP_ADDRESS"  # 替换为实际的WSL IP
port = 7000
max_connections = 1000

[tls]
cert_path = "/path/to/server.crt"  # 使用绝对路径
key_path = "/path/to/server.key"   # 使用绝对路径
verify_client = false

[auth]
tokens = ["your-secret-token"]  # 使用安全的token
require_auth = true
max_clients_per_token = 10
```

#### 3. 生成 TLS 证书
```bash
# 创建包含WSL IP的证书配置
cat > server-wsl.conf << EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = State  
L = City
O = NAT-Traversal
CN = localhost

[v3_req]
basicConstraints = CA:FALSE
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = WSL_IP_ADDRESS  # 替换为实际的WSL IP
EOF

# 生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server-wsl.conf
```

#### 4. 交叉编译 Windows Client
```bash
# 编译 Windows GUI 客户端
cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release

# 生成客户端配置
cargo run --bin nat-client -- --generate-config
```

#### 5. 配置 Client
编辑 `~/.config/nat-traversal/client.toml`:
```toml
[server]
addr = "WSL_IP_ADDRESS"  # WSL服务器地址
port = 7000
token = "your-secret-token"  # 与server配置中的token匹配
client_id = "windows-client"
auto_reconnect = true
reconnect_interval_secs = 30
tls_verify = false  # 开发环境禁用证书验证

[gui]
enabled = true
start_minimized = false
system_tray = true
theme = "dark"

[logging]
level = "info"
max_size_mb = 50
max_files = 3
```

#### 6. 部署到 Windows
```bash
# 复制必要文件到Windows可访问的位置
cp ./target/x86_64-pc-windows-gnu/release/nat-client.exe /mnt/c/temp/
cp ~/.config/nat-traversal/client.toml /mnt/c/temp/
```

### 运行指南

#### 启动 Server (WSL)
```bash
# 前台运行 (用于调试)
./target/release/nat-server

# 后台运行
nohup ./target/release/nat-server > server.log 2>&1 &

# 使用 systemd 服务 (推荐)
# 参考 platform/ 目录中的服务配置
```

#### 启动 Client (Windows)
```powershell
# GUI 模式 (双击运行或命令行)
.\nat-client.exe

# CLI 模式
.\nat-client.exe --no-gui

# 指定配置文件
.\nat-client.exe --config client.toml
```

### 网络配置

#### WSL 网络访问
WSL2 使用虚拟网络，Windows 宿主机可以直接访问 WSL IP，但需要确保：
1. WSL IP 地址可能在重启后改变，需要更新配置
2. Windows 防火墙允许访问指定端口 (7000)
3. WSL 中的 iptables 不阻止连接

#### 防火墙配置
```bash
# WSL 端检查防火墙 (如果启用)
sudo ufw status
sudo ufw allow 7000

# Windows 端添加防火墙规则 (管理员权限)
netsh advfirewall firewall add rule name="NAT Traversal Client" dir=out action=allow protocol=TCP localport=7000
```

### 故障排除

#### 常见问题
1. **连接被拒绝**: 检查 WSL IP 地址是否正确，server 是否正在运行
2. **TLS 握手失败**: 确保证书包含正确的 IP 地址，或设置 `tls_verify = false`
3. **认证失败**: 检查 server 和 client 的 token 是否一致
4. **WSL IP 变化**: 使用脚本自动更新配置或考虑使用端口转发

#### 调试命令
```bash
# 检查 server 是否监听
ss -tlnp | grep 7000

# 测试网络连接
telnet WSL_IP_ADDRESS 7000

# 查看详细日志
RUST_LOG=debug ./target/release/nat-server
RUST_LOG=debug ./nat-client.exe --no-gui
```

### 自动化脚本示例
```bash
#!/bin/bash
# wsl-setup.sh - 自动设置WSL服务器

WSL_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
echo "WSL IP: $WSL_IP"

# 更新server配置
sed -i "s/bind_addr = .*/bind_addr = \"$WSL_IP\"/" ~/.config/nat-traversal/server.toml

# 更新client配置
sed -i "s/addr = .*/addr = \"$WSL_IP\"/" ~/.config/nat-traversal/client.toml

# 重新生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=NAT-Traversal/CN=localhost" \
  -addext "subjectAltName=IP:127.0.0.1,IP:$WSL_IP"

echo "Configuration updated for WSL IP: $WSL_IP"
```

## Security Features

- Mandatory TLS encryption for all communications
- Token-based authentication
- Port range restrictions for tunnels
- Connection rate limiting
- Certificate verification (configurable)
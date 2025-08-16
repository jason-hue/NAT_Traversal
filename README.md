# NAT 穿透工具

基于 Rust 开发的高性能、跨平台 NAT 穿透解决方案。本项目通过客户端-服务器架构，实现本地服务通过 NAT 防火墙的安全隧道连接。

## 功能特性

- **跨平台支持**: 支持 Windows 和 Linux 系统
- **安全加密**: 使用 TLS 1.3 加密通信和基于令牌的身份验证
- **用户友好**: 提供基于 egui 的图形界面和命令行支持
- **高性能**: 基于 tokio 的异步架构
- **服务集成**: 原生支持 Windows 服务和 Linux systemd
- **灵活配置**: 支持 TCP 隧道协议

## 系统架构

系统由四个主要组件构成：

1. **服务器端** (`server/`): 运行在公网服务器上，管理客户端连接和端口转发
2. **客户端** (`client/`): 运行在 NAT 后的本地机器，提供图形界面和命令行接口
3. **通用库** (`common/`): 共享的协议、配置和加密功能
4. **平台集成** (`platform/`): 跨平台服务和系统集成功能

## 快速开始

### 系统要求

- Rust 1.70+ 
- Linux: GTK3 开发库 (仅 GUI 版本需要)
- Windows: 无额外依赖
- 服务器端需要 TLS 证书（测试环境可使用自签名证书）

### 安装方法

1. **克隆代码仓库**：
```bash
git clone https://github.com/yourusername/nat-traversal.git
cd nat-traversal
```

2. **安装编译依赖**：

**Linux (Ubuntu/Debian)**：
```bash
# 安装 Rust (如果未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc

# 安装 GUI 依赖（可选，仅 GUI 版本需要）
sudo apt update
sudo apt install -y libgtk-3-dev libatk1.0-dev libcairo-gobject2 \
  libcairo2-dev libgdk-pixbuf2.0-dev libgio2.0-cil-dev \
  libglib2.0-dev libpango1.0-dev pkg-config

# 安装 Windows 交叉编译工具（可选）
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

**Windows**：
```powershell
# 安装 Rust (如果未安装)
# 从 https://rustup.rs/ 下载并运行安装程序

# 无需额外系统依赖，所有功能使用纯 Rust 实现
```

3. **编译项目**：
```bash
# Linux 本地编译（包含 GUI）
cargo build --release

# Linux 编译无 GUI 版本
cargo build -p nat-traversal-client --no-default-features --release

# Windows 交叉编译（在 Linux 下）
cargo build --target x86_64-pc-windows-gnu --release

# Windows 本地编译（在 Windows 下）
cargo build --release
```

## 详细使用步骤

### 第一步：服务器端部署

#### 1.1 生成配置文件
```bash
# Linux
./target/release/nat-server --generate-config

# Windows
./target/x86_64-pc-windows-gnu/release/nat-server.exe --generate-config
# 或者在 Windows 本地编译后
./target/release/nat-server.exe --generate-config
```

**配置文件位置**：
- Linux: `~/.config/nat-traversal/server.toml`
- Windows: `%APPDATA%\nat-traversal\nat-traversal\server.toml`

#### 1.2 生成 TLS 证书

**自签名证书（测试环境）**：
```bash
# Linux
cd ~/.config/nat-traversal/
openssl genrsa -out server.key 4096
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=CN/ST=State/L=City/O=Organization/OU=Unit/CN=your-server.com"

# Windows（在配置目录中）
cd %APPDATA%\nat-traversal\nat-traversal\
openssl genrsa -out server.key 4096
openssl req -new -x509 -key server.key -out server.crt -days 365 -subj "/C=CN/ST=State/L=City/O=Organization/OU=Unit/CN=your-server.com"
```

**生产环境证书**：
使用 Let's Encrypt 或购买 SSL 证书，将证书文件放到配置目录。

#### 1.3 编辑服务器配置文件

编辑 `server.toml`：
```toml
[network]
bind_addr = "0.0.0.0"          # 监听所有网络接口
port = 7000                    # 服务端口
max_connections = 1000         # 最大连接数

[tls]
# Linux 路径示例
cert_path = "/home/user/.config/nat-traversal/server.crt"
key_path = "/home/user/.config/nat-traversal/server.key"
# Windows 路径示例
# cert_path = "C:\\Users\\username\\AppData\\Roaming\\nat-traversal\\nat-traversal\\server.crt"
# key_path = "C:\\Users\\username\\AppData\\Roaming\\nat-traversal\\nat-traversal\\server.key"
verify_client = false

[auth]
tokens = ["your-secret-token-here"]  # 修改为强密码
require_auth = true
max_clients_per_token = 10

[limits]
max_tunnels_per_client = 10
max_connections_per_tunnel = 100
connection_timeout_secs = 300

[logging]
level = "info"
max_size_mb = 100
max_files = 5
```

#### 1.4 启动服务器
```bash
# Linux
./target/release/nat-server

# Windows
./target/x86_64-pc-windows-gnu/release/nat-server.exe
# 或者在 Windows 本地编译后
./target/release/nat-server.exe

# 后台运行（Linux）
nohup ./target/release/nat-server > server.log 2>&1 &

# 使用 systemd 服务（Linux）
sudo cp nat-server.service /etc/systemd/system/
sudo systemctl enable nat-server
sudo systemctl start nat-server
```

### 第二步：客户端配置

#### 2.1 生成配置文件
```bash
# Linux
./target/release/nat-client --generate-config

# Windows
./target/x86_64-pc-windows-gnu/release/nat-client.exe --generate-config
# 或者在 Windows 本地编译后
./target/release/nat-client.exe --generate-config
```

**配置文件位置**：
- Linux: `~/.config/nat-traversal/client.toml`
- Windows: `%APPDATA%\nat-traversal\nat-traversal\client.toml`

#### 2.2 编辑客户端配置文件

编辑 `client.toml`：
```toml
[server]
addr = "your-server.com"       # 服务器公网 IP 或域名
port = 7000                    # 服务器端口
token = "your-secret-token-here"  # 与服务器配置一致
client_id = "my-desktop"       # 客户端唯一标识
auto_reconnect = true          # 自动重连
reconnect_interval_secs = 30   # 重连间隔
tls_verify = true             # 验证 TLS 证书（生产环境建议开启）

[gui]
enabled = true                # 启用图形界面
start_minimized = false       # 启动时最小化
system_tray = true           # 显示系统托盘图标
theme = "dark"               # 界面主题

[logging]
level = "info"               # 日志级别
max_size_mb = 50            # 最大日志文件大小
max_files = 3               # 保留日志文件数量

# 隧道配置将通过 GUI 管理，或手动添加：
[[tunnels]]
name = "SSH Server"          # 隧道名称
local_port = 22             # 本地端口
remote_port = 2222          # 远程端口（可选，不指定则自动分配）
protocol = "Tcp"            # 协议类型
auto_start = true           # 启动时自动连接
```

#### 2.3 启动客户端

**GUI 模式**：
```bash
# Linux
./target/release/nat-client

# Windows
./target/x86_64-pc-windows-gnu/release/nat-client.exe
# 或者在 Windows 本地编译后
./target/release/nat-client.exe
```

**命令行模式**：
```bash
# Linux
./target/release/nat-client --no-gui

# Windows
./target/x86_64-pc-windows-gnu/release/nat-client.exe --no-gui
```

**调试模式**：
```bash
# 启用详细日志
RUST_LOG=debug ./target/release/nat-client

# Windows PowerShell
$env:RUST_LOG="debug"; ./target/release/nat-client.exe
```

### 第三步：隧道管理

#### 3.1 使用 GUI 管理隧道

1. 启动 GUI 客户端
2. 在"连接配置"标签页确认服务器设置
3. 点击"连接服务器"建立连接
4. 在"隧道管理"标签页添加新隧道：
   - 隧道名称：例如 "SSH"
   - 本地端口：例如 22
   - 远程端口：例如 2222（可选）
   - 协议：TCP
5. 点击"启动隧道"

#### 3.2 常用隧道配置示例

**SSH 远程连接**：
```toml
[[tunnels]]
name = "SSH"
local_port = 22
remote_port = 2222
protocol = "Tcp"
auto_start = true
```

**Web 服务器**：
```toml
[[tunnels]]
name = "Web Server"
local_port = 8080
# remote_port 不指定，系统自动分配
protocol = "Tcp"
auto_start = false
```

**远程桌面（Windows）**：
```toml
[[tunnels]]
name = "RDP"
local_port = 3389
remote_port = 13389
protocol = "Tcp"
auto_start = true
```

**数据库连接**：
```toml
[[tunnels]]
name = "MySQL"
local_port = 3306
remote_port = 13306
protocol = "Tcp"
auto_start = false
```

### 第四步：连接测试

#### 4.1 测试隧道连接

**SSH 连接测试**：
```bash
# 通过隧道连接到本地机器
ssh user@your-server.com -p 2222
```

**Web 服务测试**：
```bash
# 假设系统分配了端口 18080
curl http://your-server.com:18080
```

**端口连通性测试**：
```bash
# Linux/Mac
telnet your-server.com 2222

# Windows
Test-NetConnection your-server.com -Port 2222
```

#### 4.2 状态监控

**查看服务器日志**：
```bash
# 实时查看日志
tail -f ~/.config/nat-traversal/server.log

# 检查错误
grep ERROR ~/.config/nat-traversal/server.log
```

**查看客户端状态**：
- GUI 模式：在状态栏查看连接状态
- CLI 模式：查看控制台输出
- 日志文件：`~/.config/nat-traversal/client.log`

### 第五步：生产部署

#### 5.1 服务器端生产配置

**使用 systemd 服务（Linux）**：
```bash
# 创建服务文件
sudo tee /etc/systemd/system/nat-server.service > /dev/null <<EOF
[Unit]
Description=NAT Traversal Server
After=network.target

[Service]
Type=simple
User=nat-server
WorkingDirectory=/opt/nat-traversal
ExecStart=/opt/nat-traversal/nat-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 启用并启动服务
sudo systemctl enable nat-server
sudo systemctl start nat-server
sudo systemctl status nat-server
```

**防火墙配置**：
```bash
# UFW (Ubuntu)
sudo ufw allow 7000/tcp

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=7000/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 7000 -j ACCEPT
```

#### 5.2 客户端自动启动

**Windows 服务安装**：
```powershell
# 使用 NSSM (Non-Sucking Service Manager)
nssm install "NAT Traversal Client" "C:\path\to\nat-client.exe"
nssm set "NAT Traversal Client" Parameters "--no-gui"
nssm start "NAT Traversal Client"
```

**Linux systemd 用户服务**：
```bash
# 创建用户服务文件
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/nat-client.service <<EOF
[Unit]
Description=NAT Traversal Client
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/bin/nat-client --no-gui
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
EOF

# 启用用户服务
systemctl --user enable nat-client
systemctl --user start nat-client
```

### 故障排除快速指南

**连接失败**：
1. 检查网络连通性：`telnet server-ip 7000`
2. 验证证书配置：`openssl s_client -connect server-ip:7000`
3. 检查防火墙设置
4. 确认认证令牌正确

**性能问题**：
1. 检查服务器资源使用：`htop`、`iotop`
2. 调整连接限制配置
3. 监控网络带宽使用

**日志分析**：
```bash
# 启用调试日志
RUST_LOG=debug ./nat-server
RUST_LOG=debug ./nat-client --no-gui

# 查看特定模块日志
RUST_LOG=nat_traversal_server::tunnel=debug ./nat-server
```

## 配置文件详解

### 服务器配置 (server.toml)

服务器配置文件包含网络、TLS、认证和限制设置：

```toml
[network]
bind_addr = "0.0.0.0"        # 服务器绑定地址
port = 7000                  # 服务器监听端口
max_connections = 1000       # 最大连接数

[tls]
cert_path = "server.crt"     # TLS 证书路径
key_path = "server.key"      # TLS 私钥路径
verify_client = false        # 是否验证客户端证书

[auth]
tokens = ["secret-token"]    # 认证令牌列表
require_auth = true          # 是否需要认证
max_clients_per_token = 10   # 每个令牌最大客户端数

[limits]
max_tunnels_per_client = 10     # 每个客户端最大隧道数
max_connections_per_tunnel = 100 # 每个隧道最大连接数
connection_timeout_secs = 300    # 连接超时时间

[logging]
level = "info"               # 日志级别
max_size_mb = 100           # 最大日志文件大小
max_files = 5               # 保留日志文件数量
```

### 客户端配置 (client.toml)

客户端配置文件包含服务器连接、GUI和隧道设置：

```toml
[server]
addr = "your-server.com"     # 服务器地址
port = 7000                  # 服务器端口
token = "secret-token"       # 认证令牌
client_id = "client-001"     # 客户端标识
auto_reconnect = true        # 自动重连
reconnect_interval_secs = 30 # 重连间隔
tls_verify = true           # 验证 TLS 证书

[gui]
enabled = true              # 启用 GUI
start_minimized = false     # 启动时最小化
system_tray = true         # 系统托盘
theme = "dark"             # 界面主题

[logging]
level = "info"             # 日志级别
max_size_mb = 50          # 最大日志文件大小
max_files = 3             # 保留日志文件数量

# 隧道配置示例
tunnels = []               # 隧道列表 (由 GUI 管理)
```

## 使用场景示例

### SSH 远程连接

配置 SSH 隧道实现远程访问：

```toml
[[tunnels]]
name = "SSH Server"
local_port = 22
remote_port = 2222
protocol = "Tcp"
auto_start = true
```

连接方式：
```bash
ssh user@your-server.com -p 2222
```

### Web 服务器访问

暴露本地 Web 服务器：

```toml
[[tunnels]]
name = "Web Server"
local_port = 8080
auto_start = false  # 手动启动
```

服务器将自动分配端口，通过 GUI 查看分配的端口号。

### 远程桌面连接

Windows 远程桌面转发：

```toml
[[tunnels]]
name = "Remote Desktop"
local_port = 3389
remote_port = 13389
protocol = "Tcp"
auto_start = true
```

## 编译和构建

### 开发环境依赖

**Linux**:
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y libgtk-3-dev libatk1.0-dev libcairo-gobject2 \
  libcairo2-dev libgdk-pixbuf2.0-dev libgio2.0-cil-dev \
  libglib2.0-dev libpango1.0-dev pkg-config

# 交叉编译到 Windows
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

**Windows**:
无额外系统依赖，所有功能使用纯 Rust 实现。

### 编译选项

```bash
# 本地编译（包含 GUI）
cargo build --release

# Windows 交叉编译
cargo build --target x86_64-pc-windows-gnu --release

# 仅命令行版本（无 GUI 依赖）
cargo build -p nat-traversal-client --no-default-features --release

# 开发版本
cargo build
```

### 特性标志

- `gui`: 启用 egui 图形界面 (默认启用)
- 使用 `--no-default-features` 可编译纯命令行版本

### 测试运行

```bash
# 运行所有测试
cargo test -p nat-traversal-common -p nat-traversal-server -p nat-traversal-platform

# 测试客户端（无 GUI）
cargo test -p nat-traversal-client --no-default-features

# 代码格式检查
cargo fmt
cargo clippy
```

## 安全特性

- **TLS 1.3 加密**: 使用 rustls 库提供的现代 TLS 实现
- **令牌认证**: 基于共享密钥的客户端身份验证
- **连接隔离**: 每个客户端的隧道完全隔离
- **证书验证**: 支持服务器证书验证
- **连接限制**: 可配置的并发连接数限制
- **超时机制**: 自动清理僵尸连接

## 故障排除

### 常见问题

1. **服务器启动失败**
```bash
# 检查证书文件
ls -la server.crt server.key

# 验证证书有效性
openssl x509 -in server.crt -text -noout

# 检查端口占用
ss -tlnp | grep 7000
```

2. **客户端连接失败**
```bash
# 测试网络连通性
telnet your-server.com 7000

# 检查 TLS 连接
openssl s_client -connect your-server.com:7000

# 查看详细日志
RUST_LOG=debug ./nat-client --no-gui
```

3. **GUI 启动失败**
```bash
# Linux: 检查 GTK 依赖
pkg-config --modversion gtk+-3.0

# 使用命令行模式
./nat-client --no-gui

# 查看错误信息
RUST_LOG=debug ./nat-client
```

4. **编译错误**
```bash
# 更新 Rust 工具链
rustup update

# 清理缓存
cargo clean

# 重新编译
cargo build --release
```

### 日志调试

启用详细日志输出：
```bash
# 服务器端调试
RUST_LOG=debug ./nat-server

# 客户端调试
RUST_LOG=debug ./nat-client

# 特定模块调试
RUST_LOG=nat_traversal_server=debug ./nat-server
```

## 开发和贡献

### 代码结构

```
nat-traversal/
├── common/          # 共享库 (协议、配置、加密)
├── server/          # 服务器端
├── client/          # 客户端 (GUI + CLI)
├── platform/        # 平台特定功能
├── target/          # 编译输出
├── CLAUDE.md        # 开发指南
├── TESTING.md       # 测试文档
└── README.md        # 本文档
```

### 开发命令

```bash
# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 运行测试
cargo test

# 生成文档
cargo doc --open
```

### 贡献指南

1. Fork 项目
2. 创建功能分支
3. 编写代码和测试
4. 运行 `cargo fmt` 和 `cargo clippy`
5. 提交 Pull Request

更多详细信息请参阅 [TESTING.md](TESTING.md)。

## 版本历史

- **v0.1.0**: 基础功能实现，TLS 通信，基本 GUI
- 更多版本计划中...

## 许可证

本项目采用双许可证：
- [MIT 许可证](LICENSE-MIT)
- [Apache-2.0 许可证](LICENSE-APACHE)

您可以选择其中任一许可证使用本项目。

## 技术支持

- **问题报告**: [GitHub Issues](https://github.com/yourusername/nat-traversal/issues)
- **功能请求**: [GitHub Discussions](https://github.com/yourusername/nat-traversal/discussions)
- **文档**: [项目 Wiki](https://github.com/yourusername/nat-traversal/wiki)

---

**感谢您使用 NAT 穿透工具！** 🚀

# NAT 穿透工具

基于 Rust 开发的高性能、跨平台 NAT 穿透解决方案。本项目通过客户端-服务器架构，实现本地服务通过 NAT 防火墙的安全隧道连接。

## 功能特性

- **跨平台支持**: 支持 Windows 11 和 Linux 系统
- **安全加密**: 使用 TLS 加密通信和基于令牌的身份验证
- **用户友好**: 提供基于 egui 的图形界面和命令行支持
- **高性能**: 基于 tokio 的异步架构
- **服务集成**: 原生支持 Windows 服务和 Linux systemd
- **灵活配置**: 支持 TCP 和 UDP 隧道协议

## 系统架构

系统由三个主要组件构成：

1. **服务器端**: 运行在公网服务器上，管理客户端连接和端口转发
2. **客户端**: 运行在 NAT 后的本地机器，提供图形界面和命令行接口
3. **通用库**: 共享的协议和配置库

## 快速开始

### 系统要求

- Rust 1.70+ 
- 服务器端需要 TLS 证书（测试环境可使用自签名证书）

### 安装方法

1. 克隆代码仓库：
```bash
git clone https://github.com/yourusername/nat-traversal.git
cd nat-traversal
```

2. 编译项目：
```bash
cargo build --release
```

### 服务器端配置

1. 生成默认配置文件：
```bash
# Linux/macOS
./target/release/nat-server --generate-config

# Windows
.\target\x86_64-pc-windows-gnu\release\nat-server.exe --generate-config
```

2. 编辑 `server.toml` 配置文件：
   - 服务器绑定地址和端口
   - TLS 证书路径
   - 认证令牌

3. 生成 TLS 证书（测试用）：
```bash
# 生成自签名证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -subj "/C=CN/ST=Test/L=Test/O=Test/OU=Test/CN=localhost"
```

4. 启动服务器：
```bash
# Linux/macOS
./target/release/nat-server

# Windows
.\target\x86_64-pc-windows-gnu\release\nat-server.exe
```

### 客户端配置

1. 生成默认配置文件：
```bash
# Linux/macOS
./target/release/nat-client --generate-config

# Windows
.\target\x86_64-pc-windows-gnu\release\nat-client.exe --generate-config
```

2. 编辑 `client.toml` 配置文件：
   - 服务器地址和认证令牌
   - 隧道配置

3. 启动客户端（图形界面）：
```bash
# Linux/macOS
./target/release/nat-client

# Windows
.\target\x86_64-pc-windows-gnu\release\nat-client.exe
```

或使用命令行模式：
```bash
# Linux/macOS
./target/release/nat-client --no-gui

# Windows
.\target\x86_64-pc-windows-gnu\release\nat-client.exe --no-gui
```

## 配置文件详解

### 服务器配置 (server.toml)

```toml
[network]
bind_addr = "0.0.0.0"    # 服务器绑定地址
port = 7000              # 服务器监听端口
max_connections = 1000   # 最大连接数

[tls]
cert_path = "server.crt" # TLS 证书路径
key_path = "server.key"  # TLS 私钥路径
verify_client = false    # 是否验证客户端证书

[auth]
tokens = ["your-secret-token"]  # 认证令牌列表
require_auth = true             # 是否需要认证
max_clients_per_token = 10      # 每个令牌最大客户端数

[limits]
max_tunnels_per_client = 10     # 每个客户端最大隧道数
max_connections_per_tunnel = 100 # 每个隧道最大连接数
connection_timeout_secs = 300    # 连接超时时间（秒）
```

### 客户端配置 (client.toml)

```toml
[server]
addr = "your-server.com"  # 服务器地址
port = 7000               # 服务器端口
token = "your-secret-token" # 认证令牌
client_id = "my-client"   # 客户端标识
auto_reconnect = true     # 自动重连
tls_verify = true         # 验证 TLS 证书

[[tunnels]]
name = "SSH"              # 隧道名称
local_port = 22           # 本地端口
remote_port = 2222        # 远程端口（可选，自动分配）
protocol = "Tcp"          # 协议类型（Tcp/Udp）
auto_start = true         # 自动启动

[[tunnels]]
name = "Web Server"       # Web 服务器隧道
local_port = 8080         # 本地 Web 服务端口
auto_start = false        # 手动启动
```

## 使用场景示例

### SSH 远程连接

将本地 SSH 服务（22端口）转发到远程服务器的 2222 端口：

```bash
# 服务器将监听 2222 端口
# 连接到 server:2222 的流量将转发到 client:22
ssh user@your-server.com -p 2222
```

### Web 服务器访问

暴露本地运行在 8080 端口的 Web 服务器：

```bash
# 服务器将自动分配端口（例如 8001）
# 访问 http://your-server.com:8001 将转发到本地 8080 端口
```

### 远程桌面连接

转发 Windows 远程桌面服务：

```toml
[[tunnels]]
name = "Remote Desktop"
local_port = 3389
remote_port = 13389
protocol = "Tcp"
auto_start = true
```

## 跨平台支持

### Windows 平台

- 原生 Windows 服务支持
- 系统托盘集成
- MSI 安装包（计划中）
- 图形界面完全支持

#### Windows 编译要求

```bash
# 安装交叉编译工具链
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu

# 编译 Windows 版本
cargo build --target x86_64-pc-windows-gnu --release
```

### Linux 平台

- systemd 服务集成
- 包管理器支持（deb/rpm 计划中）
- AppImage 分发（计划中）

#### Linux GUI 依赖

```bash
# Ubuntu/Debian
sudo apt install libgtk-3-dev libatk1.0-dev libcairo-gobject2 \
  libcairo2-dev libgdk-pixbuf2.0-dev libgio2.0-cil-dev \
  libglib2.0-dev libpango1.0-dev pkg-config

# CentOS/RHEL
sudo yum install gtk3-devel atk-devel cairo-gobject-devel \
  cairo-devel gdk-pixbuf2-devel glib2-devel pango-devel pkgconfig
```

## 安全特性

- **TLS 1.3 加密**: 所有通信均使用 TLS 加密
- **令牌认证**: 基于令牌的客户端身份验证
- **连接隔离**: 每个客户端的隧道相互隔离
- **速率限制**: 可配置的连接数和带宽限制
- **证书验证**: 支持客户端和服务器证书验证

## 编译构建

### 开发版本编译
```bash
cargo build
```

### 发布版本编译
```bash
cargo build --release
```

### 仅编译命令行版本（无 GUI 依赖）
```bash
cargo build -p nat-traversal-client --no-default-features --release
```

### 跨平台编译

从 Linux 编译 Windows 版本：
```bash
cargo build --target x86_64-pc-windows-gnu --release
```

从 Windows/WSL 编译 Linux 版本：
```bash
cargo build --target x86_64-unknown-linux-gnu --release
```

### 测试运行
```bash
# 运行通用库测试
cargo test -p nat-traversal-common

# 运行服务器测试
cargo test -p nat-traversal-server

# 运行客户端测试（无 GUI）
cargo test -p nat-traversal-client --no-default-features
```

## 故障排除

### 常见问题

1. **服务器启动失败 - 证书文件未找到**
   ```bash
   # 确保证书文件存在
   ls -la server.crt server.key
   
   # 重新生成证书
   openssl genrsa -out server.key 2048
   openssl req -new -x509 -key server.key -out server.crt -days 365 -subj "/CN=localhost"
   ```

2. **客户端连接失败**
   ```bash
   # 检查服务器地址和端口
   telnet your-server.com 7000
   
   # 检查防火墙设置
   sudo ufw allow 7000/tcp  # Ubuntu
   sudo firewall-cmd --add-port=7000/tcp --permanent  # CentOS
   ```

3. **Windows GUI 闪退**
   ```powershell
   # 在 PowerShell 中运行查看错误信息
   .\nat-client.exe
   
   # 或使用命令行模式
   .\nat-client.exe --no-gui
   ```

4. **Linux GUI 编译失败**
   ```bash
   # 安装 GTK 开发库
   sudo apt install libgtk-3-dev pkg-config
   
   # 或编译无 GUI 版本
   cargo build -p nat-traversal-client --no-default-features
   ```

### 日志调试

启用详细日志输出：
```bash
# 服务器端
RUST_LOG=debug ./nat-server

# 客户端
RUST_LOG=debug ./nat-client
```

## 如何贡献

我们欢迎所有形式的贡献！请按照以下步骤参与项目：

### 准备开发环境

1. **Fork 项目仓库**
   - 在 GitHub 上 fork 本项目到您的账户

2. **克隆代码**
   ```bash
   git clone https://github.com/your-username/nat-traversal.git
   cd nat-traversal
   ```

3. **安装开发依赖**
   ```bash
   # 安装 Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # 安装代码格式化工具
   rustup component add rustfmt clippy
   
   # Linux 下安装 GUI 依赖
   sudo apt install libgtk-3-dev pkg-config
   ```

### 开发流程

1. **创建功能分支**
   ```bash
   git checkout -b feature/new-feature
   # 或
   git checkout -b fix/bug-description
   ```

2. **编写代码**
   - 遵循现有代码风格
   - 添加必要的注释
   - 确保代码安全性

3. **运行测试**
   ```bash
   # 格式化代码
   cargo fmt
   
   # 代码检查
   cargo clippy
   
   # 运行测试
   cargo test
   
   # 编译检查
   cargo build --release
   ```

4. **提交代码**
   ```bash
   git add .
   git commit -m "功能: 添加新的隧道协议支持"
   ```

5. **推送分支**
   ```bash
   git push origin feature/new-feature
   ```

6. **创建 Pull Request**
   - 在 GitHub 上创建 PR
   - 详细描述您的更改
   - 等待代码审查

### 贡献类型

#### 🚀 功能增强
- 新的隧道协议支持
- 性能优化
- 用户界面改进
- 平台特定功能

#### 🐛 Bug 修复
- 连接稳定性问题
- 内存泄漏修复
- 跨平台兼容性
- 安全漏洞修复

#### 📚 文档改进
- API 文档
- 使用指南
- 示例代码
- 多语言翻译

#### 🧪 测试完善
- 单元测试
- 集成测试
- 性能测试
- 端到端测试

### 代码规范

1. **Rust 代码风格**
   ```bash
   # 使用 rustfmt 格式化
   cargo fmt
   
   # 使用 clippy 检查
   cargo clippy -- -D warnings
   ```

2. **提交信息格式**
   ```
   类型: 简短描述（50字符以内）
   
   详细说明（如需要）
   - 解决的问题
   - 实现的功能
   - 影响范围
   ```

3. **安全要求**
   - 不提交敏感信息（密钥、令牌等）
   - 验证输入数据
   - 使用安全的依赖库版本
   - 遵循最佳安全实践

### 问题报告

发现 Bug 或有功能建议？请：

1. **搜索现有 Issue** - 避免重复报告
2. **使用 Issue 模板** - 提供完整信息
3. **包含系统信息** - 操作系统、Rust 版本等
4. **提供复现步骤** - 详细的复现方法

### 社区行为准则

- 尊重所有贡献者
- 使用友善的语言
- 专注于技术讨论
- 欢迎不同观点

## 版本发布

### 版本号规则

我们遵循 [语义化版本](https://semver.org/) 规范：

- `MAJOR.MINOR.PATCH` (例如: 1.0.0)
- MAJOR: 不兼容的 API 变更
- MINOR: 向后兼容的功能新增
- PATCH: 向后兼容的 Bug 修复

### 发布计划

- **v0.1.0**: 基础功能实现
- **v0.2.0**: GUI 界面完善
- **v0.3.0**: 服务集成支持
- **v1.0.0**: 稳定版本发布

## 许可证

本项目采用双许可证：

- **MIT 许可证** - 详见 [LICENSE-MIT](LICENSE-MIT)
- **Apache-2.0 许可证** - 详见 [LICENSE-APACHE](LICENSE-APACHE)

您可以选择其中任一许可证使用本项目。

## 技术支持

### 获取帮助

- **GitHub Issues**: 报告 Bug 和功能请求
- **GitHub Discussions**: 社区讨论和技术交流
- **Wiki**: 详细文档和教程

### 联系方式

- 项目维护者: [Your Name](mailto:your-email@example.com)
- 官方网站: https://your-project-website.com
- 社区论坛: https://forum.your-project.com

---

**感谢您对 NAT 穿透工具的关注和支持！** 🚀
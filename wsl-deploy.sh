#!/bin/bash
# wsl-deploy.sh - WSL + Windows 自动化部署脚本

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== NAT Traversal WSL + Windows 自动部署 ===${NC}"
echo ""

# 检查环境
if ! grep -q microsoft /proc/version; then
    echo -e "${RED}错误: 此脚本必须在 WSL 环境中运行${NC}"
    exit 1
fi

# 获取 WSL IP
WSL_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
if [ -z "$WSL_IP" ]; then
    echo -e "${RED}错误: 无法获取 WSL IP 地址${NC}"
    exit 1
fi

echo -e "${GREEN}检测到 WSL IP: $WSL_IP${NC}"

# 生成安全 token
SECURE_TOKEN="wsl-$(date +%Y%m%d)-$(openssl rand -hex 8)"
echo -e "${GREEN}生成安全 token: $SECURE_TOKEN${NC}"

# 1. 构建项目
echo ""
echo -e "${BLUE}1. 构建项目...${NC}"

echo "构建服务器..."
cargo build --bin nat-server --release
echo -e "${GREEN}✓ 服务器构建完成${NC}"

echo "构建客户端..."
cargo build --bin nat-client --release  
echo -e "${GREEN}✓ 客户端构建完成${NC}"

# 检查 Windows 交叉编译工具
if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "交叉编译 Windows 客户端..."
    cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release
    echo -e "${GREEN}✓ Windows 客户端构建完成${NC}"
    WINDOWS_CLIENT_AVAILABLE=true
else
    echo -e "${YELLOW}! Windows 交叉编译工具未安装，跳过 Windows 客户端构建${NC}"
    echo "  安装命令: sudo apt install gcc-mingw-w64-x86-64"
    WINDOWS_CLIENT_AVAILABLE=false
fi

# 2. 生成配置
echo ""
echo -e "${BLUE}2. 生成配置文件...${NC}"

# 生成基础配置
./target/release/nat-server --generate-config >/dev/null 2>&1
./target/release/nat-client --generate-config >/dev/null 2>&1

# 更新服务器配置
cat > ~/.config/nat-traversal/server.toml << EOF
[network]
bind_addr = "$WSL_IP"
port = 7000
max_connections = 1000

[tls]
cert_path = "$(pwd)/server.crt"
key_path = "$(pwd)/server.key"
verify_client = false

[auth]
tokens = ["$SECURE_TOKEN"]
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
EOF

# 更新客户端配置
cat > ~/.config/nat-traversal/client.toml << EOF
[server]
addr = "$WSL_IP"
port = 7000
token = "$SECURE_TOKEN"
client_id = "windows-client"
auto_reconnect = true
reconnect_interval_secs = 30
tls_verify = false

[gui]
enabled = true
start_minimized = false
system_tray = true
theme = "dark"

[logging]
level = "info"
max_size_mb = 50
max_files = 3

tunnels = []
EOF

echo -e "${GREEN}✓ 配置文件已生成并配置为 WSL IP: $WSL_IP${NC}"

# 3. 生成 TLS 证书
echo ""
echo -e "${BLUE}3. 生成 TLS 证书...${NC}"

cat > server-wsl.conf << EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = Development
L = WSL
O = NAT-Traversal
CN = wsl-server

[v3_req]
basicConstraints = CA:FALSE
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = wsl-server  
IP.1 = 127.0.0.1
IP.2 = $WSL_IP
EOF

openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server-wsl.conf

echo -e "${GREEN}✓ TLS 证书已生成${NC}"

# 4. 部署到 Windows
if [ "$WINDOWS_CLIENT_AVAILABLE" = true ]; then
    echo ""
    echo -e "${BLUE}4. 部署到 Windows...${NC}"
    
    # 创建 Windows 目录
    WINDOWS_DIR="/mnt/c/NAT-Traversal"
    mkdir -p "$WINDOWS_DIR"
    
    # 复制文件
    cp ./target/x86_64-pc-windows-gnu/release/nat-client.exe "$WINDOWS_DIR/"
    cp ~/.config/nat-traversal/client.toml "$WINDOWS_DIR/"
    
    # 创建启动脚本
    cat > "$WINDOWS_DIR/start-gui.bat" << 'EOF'
@echo off
cd /d "%~dp0"
echo NAT Traversal Client - GUI Mode
echo ================================
echo.
echo Starting client... (Press Ctrl+C to stop)
echo If connection fails, make sure WSL server is running.
echo.
nat-client.exe --config client.toml
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Failed to start client
    echo Check that WSL server is running and accessible
)
pause
EOF
    
    cat > "$WINDOWS_DIR/start-cli.bat" << 'EOF'
@echo off
cd /d "%~dp0"
echo NAT Traversal Client - CLI Mode  
echo =================================
echo.
echo Starting client... (Press Ctrl+C to stop)
echo.
nat-client.exe --config client.toml --no-gui
EOF
    
    # 创建配置文件查看器
    cat > "$WINDOWS_DIR/show-config.bat" << 'EOF'
@echo off
echo Current client configuration:
echo =============================
type client.toml
echo.
echo =============================
pause
EOF
    
    echo -e "${GREEN}✓ Windows 客户端已部署到: C:\\NAT-Traversal\\${NC}"
    echo "  - nat-client.exe: Windows 客户端程序"
    echo "  - client.toml: 客户端配置文件"
    echo "  - start-gui.bat: GUI 模式启动脚本"
    echo "  - start-cli.bat: CLI 模式启动脚本"
    echo "  - show-config.bat: 查看配置脚本"
fi

# 5. 生成使用说明
cat > usage-instructions.txt << EOF
NAT Traversal WSL + Windows 部署完成！

=== 环境信息 ===
WSL IP: $WSL_IP
服务器端口: 7000
认证 Token: $SECURE_TOKEN

=== 使用说明 ===

1. 启动 WSL 服务器:
   cd $(pwd)
   ./target/release/nat-server

2. 启动 Windows 客户端:
   方法1: 双击 C:\\NAT-Traversal\\start-gui.bat (GUI 模式)
   方法2: 双击 C:\\NAT-Traversal\\start-cli.bat (CLI 模式)
   方法3: 命令行运行 C:\\NAT-Traversal\\nat-client.exe

3. 调试模式:
   WSL 服务器: RUST_LOG=debug ./target/release/nat-server
   Windows 客户端: 在批处理文件前加 set RUST_LOG=debug

=== 常见问题 ===

1. 连接失败: 
   - 检查 WSL 服务器是否运行
   - 确认 WSL IP 地址未改变
   - 检查 Windows 防火墙设置

2. TLS 错误:
   - 配置已设置为 tls_verify = false
   - 如仍有问题，检查证书文件是否存在

3. WSL IP 变化:
   - 重新运行此脚本更新配置
   - 或手动更新配置文件中的 IP 地址

=== 文件位置 ===
- WSL 服务器配置: ~/.config/nat-traversal/server.toml  
- WSL 客户端配置: ~/.config/nat-traversal/client.toml
- Windows 客户端目录: C:\\NAT-Traversal\\
- TLS 证书: $(pwd)/server.crt, $(pwd)/server.key

EOF

echo ""
echo -e "${BLUE}=== 部署完成 ===${NC}"
echo -e "${GREEN}✓ 所有组件已成功构建和配置${NC}"
echo ""
echo "使用说明已保存到: usage-instructions.txt"
echo ""
echo -e "${YELLOW}下一步:${NC}"
echo "1. 启动服务器: ./target/release/nat-server"
echo "2. 在 Windows 中运行: C:\\NAT-Traversal\\start-gui.bat"
echo ""
echo "详细文档请参阅:"
echo "- README.md: 基础使用指南"
echo "- DEPLOYMENT.md: 详细部署文档"
echo "- CLAUDE.md: 开发者指南"
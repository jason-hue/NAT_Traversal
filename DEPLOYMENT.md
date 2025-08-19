# NAT 穿透工具部署指南

本文档提供NAT穿透工具的完整部署指南，包括各种部署场景和详细的故障排除方法。

## 目录

- [快速开始](#快速开始)
- [WSL + Windows 部署](#wsl--windows-部署)
- [Linux 服务器部署](#linux-服务器部署)
- [Windows 服务器部署](#windows-服务器部署)
- [TLS 证书配置](#tls-证书配置)
- [网络和防火墙配置](#网络和防火墙配置)
- [故障排除](#故障排除)
- [性能优化](#性能优化)

## 快速开始

### 最简部署（开发测试）

适用于本地测试和开发环境。

#### 1. 编译项目
```bash
# 克隆项目
git clone <repository-url>
cd NAT_Traversal

# Linux 编译
cargo build --release

# Windows 交叉编译
sudo apt install gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release
```

#### 2. 生成配置和证书
```bash
# 生成配置文件
cargo run --bin nat-server -- --generate-config
cargo run --bin nat-client -- --generate-config

# 生成自签名证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=NAT-Traversal/CN=localhost"

# 修改客户端配置为不验证证书
sed -i 's/tls_verify = true/tls_verify = false/' ~/.config/nat-traversal/client.toml
```

#### 3. 运行测试
```bash
# 启动服务器
./target/release/nat-server &

# 启动客户端
./target/release/nat-client --no-gui
```

## WSL + Windows 部署

### 场景描述
- WSL2 环境作为服务器
- Windows 宿主机作为客户端
- 适合开发和测试环境

### 详细步骤

#### 1. WSL 环境准备
```bash
# 检查 WSL 版本（确保是 WSL2）
wsl --version

# 获取 WSL IP 地址
WSL_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
echo "WSL IP: $WSL_IP"  # 例如：172.22.247.72

# 安装依赖
sudo apt update
sudo apt install -y gcc-mingw-w64-x86-64 build-essential pkg-config openssl
rustup target add x86_64-pc-windows-gnu
```

#### 2. 服务器配置
```bash
# 生成服务器配置
cargo run --bin nat-server -- --generate-config

# 编辑配置文件绑定到 WSL IP
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
tokens = ["wsl-dev-token-$(date +%s)"]
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
```

#### 3. TLS 证书配置
```bash
# 生成包含 WSL IP 的证书
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

# 生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server-wsl.conf

# 验证证书
openssl x509 -in server.crt -text -noout | grep -A1 "Subject Alternative Name"
```

#### 4. 客户端配置
```bash
# 生成客户端配置
cargo run --bin nat-client -- --generate-config

# 获取服务器 token
SERVER_TOKEN=$(grep -o '"[^"]*"' ~/.config/nat-traversal/server.toml | head -1 | tr -d '"')

# 配置客户端
cat > ~/.config/nat-traversal/client.toml << EOF
[server]
addr = "$WSL_IP"
port = 7000
token = "$SERVER_TOKEN"
client_id = "windows-client-$(hostname)"
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

tunnels = []
EOF
```

#### 5. 编译和部署
```bash
# 编译服务器
cargo build --bin nat-server --release

# 编译 Windows 客户端
cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release

# 创建 Windows 部署目录
mkdir -p /mnt/c/NAT-Traversal

# 复制文件
cp ./target/x86_64-pc-windows-gnu/release/nat-client.exe /mnt/c/NAT-Traversal/
cp ~/.config/nat-traversal/client.toml /mnt/c/NAT-Traversal/

# 创建启动脚本
cat > /mnt/c/NAT-Traversal/start-gui.bat << 'EOF'
@echo off
cd /d "%~dp0"
echo Starting NAT Traversal Client (GUI Mode)...
echo Server: Check WSL for server status
echo.
nat-client.exe --config client.toml
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Client failed to start. Check configuration.
    echo Make sure WSL server is running.
)
pause
EOF

cat > /mnt/c/NAT-Traversal/start-cli.bat << 'EOF'
@echo off
cd /d "%~dp0"
echo Starting NAT Traversal Client (CLI Mode)...
echo Press Ctrl+C to stop
echo.
nat-client.exe --config client.toml --no-gui
EOF
```

#### 6. 运行测试
```bash
# WSL 中启动服务器
echo "Starting server on $WSL_IP:7000"
./target/release/nat-server

# 在 Windows 中运行客户端
# 双击 C:\NAT-Traversal\start-gui.bat
```

### WSL 部署常见问题

#### WSL IP 地址变化
WSL2 的 IP 地址在重启后可能变化，可以使用自动化脚本：

```bash
#!/bin/bash
# wsl-auto-update.sh
set -e

echo "=== WSL NAT Traversal Auto Update ==="

# 获取当前 WSL IP
NEW_WSL_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
OLD_WSL_IP=$(grep bind_addr ~/.config/nat-traversal/server.toml | cut -d'"' -f2)

if [ "$NEW_WSL_IP" != "$OLD_WSL_IP" ]; then
    echo "WSL IP changed from $OLD_WSL_IP to $NEW_WSL_IP"
    
    # 更新服务器配置
    sed -i "s/bind_addr = \"$OLD_WSL_IP\"/bind_addr = \"$NEW_WSL_IP\"/" ~/.config/nat-traversal/server.toml
    
    # 更新客户端配置
    sed -i "s/addr = \"$OLD_WSL_IP\"/addr = \"$NEW_WSL_IP\"/" ~/.config/nat-traversal/client.toml
    
    # 重新生成证书
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
IP.1 = 127.0.0.1
IP.2 = $NEW_WSL_IP
EOF
    
    openssl genrsa -out server.key 2048
    openssl req -new -x509 -key server.key -out server.crt -days 365 -config server-wsl.conf
    
    # 复制更新的客户端配置到 Windows
    cp ~/.config/nat-traversal/client.toml /mnt/c/NAT-Traversal/
    
    echo "Configuration updated for new WSL IP: $NEW_WSL_IP"
    echo "Please restart the server and client."
else
    echo "WSL IP unchanged: $NEW_WSL_IP"
fi
```

#### Windows 网络访问问题
```bash
# 测试 WSL 到 Windows 网络连接
ping 172.22.240.1  # Windows 宿主机 IP

# 测试端口连通性
nc -zv $WSL_IP 7000

# 检查 Windows 防火墙
# 在 Windows PowerShell 中运行：
# Test-NetConnection -ComputerName 172.22.247.72 -Port 7000
```

## Linux 服务器部署

### 生产环境部署

适用于生产服务器和 VPS 部署。

#### 1. 系统准备
```bash
# 创建专用用户
sudo useradd -r -s /bin/false nat-server
sudo mkdir -p /opt/nat-traversal
sudo mkdir -p /var/log/nat-traversal
sudo chown nat-server:nat-server /var/log/nat-traversal
```

#### 2. 编译和安装
```bash
# 编译 release 版本
cargo build --release --bin nat-server

# 安装到系统目录
sudo cp target/release/nat-server /opt/nat-traversal/
sudo chown nat-server:nat-server /opt/nat-traversal/nat-server
sudo chmod 755 /opt/nat-traversal/nat-server
```

#### 3. 配置文件
```bash
# 创建配置目录
sudo mkdir -p /etc/nat-traversal
sudo chown nat-server:nat-server /etc/nat-traversal

# 生成生产配置
sudo -u nat-server tee /etc/nat-traversal/server.toml > /dev/null << EOF
[network]
bind_addr = "0.0.0.0"
port = 7000
max_connections = 1000

[tls]
cert_path = "/etc/nat-traversal/server.crt"
key_path = "/etc/nat-traversal/server.key"
verify_client = false

[auth]
tokens = ["$(openssl rand -base64 32)"]
require_auth = true
max_clients_per_token = 100

[limits]
max_tunnels_per_client = 20
max_connections_per_tunnel = 200
connection_timeout_secs = 300

[logging]
level = "info"
file = "/var/log/nat-traversal/server.log"
max_size_mb = 100
max_files = 10
EOF
```

#### 4. TLS 证书（Let's Encrypt）
```bash
# 安装 certbot
sudo apt update
sudo apt install certbot

# 获取证书（需要域名）
sudo certbot certonly --standalone -d your-domain.com

# 复制证书到配置目录
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem /etc/nat-traversal/server.crt
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem /etc/nat-traversal/server.key
sudo chown nat-server:nat-server /etc/nat-traversal/server.*

# 设置自动更新
sudo crontab -e
# 添加：0 3 * * * certbot renew --post-hook "systemctl restart nat-server"
```

#### 5. Systemd 服务
```bash
# 创建服务文件
sudo tee /etc/systemd/system/nat-server.service > /dev/null << EOF
[Unit]
Description=NAT Traversal Server
Documentation=https://github.com/your-repo/nat-traversal
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=nat-server
Group=nat-server
WorkingDirectory=/opt/nat-traversal
ExecStart=/opt/nat-traversal/nat-server --config /etc/nat-traversal/server.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/log/nat-traversal

[Install]
WantedBy=multi-user.target
EOF

# 启用服务
sudo systemctl daemon-reload
sudo systemctl enable nat-server
sudo systemctl start nat-server
sudo systemctl status nat-server
```

#### 6. 防火墙配置
```bash
# UFW
sudo ufw allow 7000/tcp
sudo ufw status

# firewalld
sudo firewall-cmd --permanent --add-port=7000/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 7000 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/rules.v4
```

#### 7. 监控和日志
```bash
# 查看服务状态
sudo systemctl status nat-server

# 查看日志
sudo journalctl -u nat-server -f

# 查看应用日志
sudo tail -f /var/log/nat-traversal/server.log

# 监控资源使用
htop
ss -tlnp | grep 7000
```

## Windows 服务器部署

### 使用 NSSM 部署 Windows 服务

#### 1. 编译 Windows 版本
```bash
# 在 Linux 中交叉编译
cargo build --target x86_64-pc-windows-gnu --release --bin nat-server

# 或在 Windows 中直接编译
cargo build --release --bin nat-server
```

#### 2. 配置文件
在 Windows 中创建 `C:\NAT-Traversal\server.toml`：
```toml
[network]
bind_addr = "0.0.0.0"
port = 7000
max_connections = 1000

[tls]
cert_path = "C:\\NAT-Traversal\\server.crt"
key_path = "C:\\NAT-Traversal\\server.key"
verify_client = false

[auth]
tokens = ["windows-server-token-2024"]
require_auth = true
max_clients_per_token = 50

[limits]
max_tunnels_per_client = 15
max_connections_per_tunnel = 150
connection_timeout_secs = 300

[logging]
level = "info"
file = "C:\\NAT-Traversal\\logs\\server.log"
max_size_mb = 100
max_files = 5
```

#### 3. 使用 NSSM 创建服务
```powershell
# 下载并安装 NSSM
# 从 https://nssm.cc/download 下载 NSSM

# 管理员权限运行 PowerShell
# 创建服务
nssm install "NAT Traversal Server" "C:\NAT-Traversal\nat-server.exe"
nssm set "NAT Traversal Server" Parameters "--config C:\NAT-Traversal\server.toml"
nssm set "NAT Traversal Server" DisplayName "NAT Traversal Server"
nssm set "NAT Traversal Server" Description "High-performance NAT traversal server"
nssm set "NAT Traversal Server" Start SERVICE_AUTO_START

# 配置日志
nssm set "NAT Traversal Server" AppStdout "C:\NAT-Traversal\logs\stdout.log"
nssm set "NAT Traversal Server" AppStderr "C:\NAT-Traversal\logs\stderr.log"

# 启动服务
nssm start "NAT Traversal Server"

# 检查服务状态
Get-Service "NAT Traversal Server"
```

## TLS 证书配置

### 开发环境自签名证书

#### 基础自签名证书
```bash
# 生成私钥
openssl genrsa -out server.key 2048

# 生成证书
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=Development/L=Local/O=NAT-Traversal/CN=localhost"
```

#### 高级自签名证书（推荐）
```bash
# 创建证书配置文件
cat > server.conf << EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = Development
L = Local
O = NAT-Traversal
CN = nat-server

[v3_req]
basicConstraints = CA:FALSE
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = nat-server
DNS.3 = *.local
IP.1 = 127.0.0.1
IP.2 = 192.168.1.100  # 替换为你的服务器 IP
IP.3 = 10.0.0.100     # 如果有多个网络接口
EOF

# 生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 -config server.conf

# 验证证书
openssl x509 -in server.crt -text -noout
```

### 生产环境证书

#### Let's Encrypt 证书
```bash
# 安装 certbot
sudo apt update && sudo apt install certbot

# 获取证书（HTTP 验证）
sudo certbot certonly --standalone -d your-domain.com

# 获取证书（DNS 验证）
sudo certbot certonly --manual --preferred-challenges dns -d your-domain.com

# 证书路径
# 证书：/etc/letsencrypt/live/your-domain.com/fullchain.pem
# 私钥：/etc/letsencrypt/live/your-domain.com/privkey.pem

# 自动续期
sudo crontab -e
# 添加：0 3 1 * * certbot renew --post-hook "systemctl restart nat-server"
```

#### 商业证书配置
```bash
# 如果你有商业证书，确保证书链完整
cat your-certificate.crt intermediate.crt > server.crt
cp your-private.key server.key

# 验证证书链
openssl verify -CAfile ca-bundle.crt server.crt
```

### 客户端证书验证配置

#### 禁用证书验证（开发环境）
```toml
# client.toml
[server]
tls_verify = false
```

#### 启用证书验证（生产环境）
```toml
# client.toml
[server]
tls_verify = true
```

## 网络和防火墙配置

### Linux 防火墙配置

#### UFW (Ubuntu/Debian)
```bash
# 基本设置
sudo ufw enable
sudo ufw default deny incoming
sudo ufw default allow outgoing

# 允许 SSH（重要：避免被锁定）
sudo ufw allow ssh

# 允许 NAT Traversal
sudo ufw allow 7000/tcp
sudo ufw allow 7000:7100/tcp  # 如果使用端口范围

# 查看状态
sudo ufw status verbose
```

#### firewalld (CentOS/RHEL/Fedora)
```bash
# 基本设置
sudo systemctl enable firewalld
sudo systemctl start firewalld

# 添加服务端口
sudo firewall-cmd --permanent --add-port=7000/tcp
sudo firewall-cmd --permanent --add-port=7000-7100/tcp

# 创建自定义服务
sudo tee /etc/firewalld/services/nat-traversal.xml > /dev/null << EOF
<?xml version="1.0" encoding="utf-8"?>
<service>
  <short>NAT Traversal</short>
  <description>NAT Traversal Server</description>
  <port protocol="tcp" port="7000"/>
</service>
EOF

sudo firewall-cmd --permanent --add-service=nat-traversal
sudo firewall-cmd --reload
sudo firewall-cmd --list-all
```

#### iptables
```bash
# 添加规则
sudo iptables -A INPUT -p tcp --dport 7000 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 7000:7100 -j ACCEPT

# 保存规则（Ubuntu/Debian）
sudo apt install iptables-persistent
sudo iptables-save | sudo tee /etc/iptables/rules.v4

# 保存规则（CentOS/RHEL）
sudo service iptables save
```

### Windows 防火墙配置

#### 使用 PowerShell（管理员权限）
```powershell
# 允许入站连接
New-NetFirewallRule -DisplayName "NAT Traversal Server" -Direction Inbound -Protocol TCP -LocalPort 7000 -Action Allow

# 允许出站连接
New-NetFirewallRule -DisplayName "NAT Traversal Client" -Direction Outbound -Protocol TCP -RemotePort 7000 -Action Allow

# 查看规则
Get-NetFirewallRule -DisplayName "NAT Traversal*"
```

#### 使用 netsh
```cmd
REM 添加入站规则
netsh advfirewall firewall add rule name="NAT Traversal Server" dir=in action=allow protocol=TCP localport=7000

REM 添加出站规则  
netsh advfirewall firewall add rule name="NAT Traversal Client" dir=out action=allow protocol=TCP remoteport=7000

REM 查看规则
netsh advfirewall firewall show rule name="NAT Traversal Server"
```

### 网络诊断

#### 连接测试
```bash
# 测试端口连通性
telnet server-ip 7000
nc -zv server-ip 7000

# 测试 TLS 连接
openssl s_client -connect server-ip:7000

# 使用 nmap 扫描
nmap -p 7000 server-ip
```

#### 网络排查
```bash
# 查看网络接口
ip addr show
ifconfig

# 查看路由表
ip route show
route -n

# 查看监听端口
ss -tlnp | grep 7000
netstat -tlnp | grep 7000

# 查看连接状态
ss -tnp | grep 7000
netstat -tnp | grep 7000
```

## 故障排除

### 常见连接问题

#### 1. 连接被拒绝 (Connection Refused)

**可能原因：**
- 服务器未启动
- 端口未监听
- 防火墙阻止连接

**排查步骤：**
```bash
# 检查服务器进程
ps aux | grep nat-server
systemctl status nat-server

# 检查端口监听
ss -tlnp | grep 7000
lsof -i :7000

# 测试本地连接
telnet localhost 7000

# 检查防火墙
sudo ufw status
sudo iptables -L
```

**解决方法：**
```bash
# 启动服务器
./nat-server
systemctl start nat-server

# 检查配置文件
cat ~/.config/nat-traversal/server.toml

# 修改防火墙
sudo ufw allow 7000/tcp
```

#### 2. TLS 握手失败

**错误信息：**
- `TLS handshake failed: invalid peer certificate`
- `certificate verify failed`
- `unknown certificate`

**排查步骤：**
```bash
# 测试 TLS 连接
openssl s_client -connect server-ip:7000

# 检查证书有效性
openssl x509 -in server.crt -text -noout
openssl x509 -in server.crt -noout -dates

# 检查证书和私钥匹配
openssl x509 -noout -modulus -in server.crt | openssl md5
openssl rsa -noout -modulus -in server.key | openssl md5
```

**解决方法：**
```bash
# 重新生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=Org/CN=your-server-ip"

# 或者禁用客户端证书验证
sed -i 's/tls_verify = true/tls_verify = false/' ~/.config/nat-traversal/client.toml
```

#### 3. 认证失败

**错误信息：**
- `Authentication failed`
- `Invalid token`

**排查步骤：**
```bash
# 检查服务器配置中的 token
grep -A5 "\[auth\]" ~/.config/nat-traversal/server.toml

# 检查客户端配置中的 token
grep "token" ~/.config/nat-traversal/client.toml

# 查看服务器日志
tail -f /var/log/nat-traversal/server.log
journalctl -u nat-server -f
```

**解决方法：**
```bash
# 确保 token 一致
SERVER_TOKEN=$(grep -o '"[^"]*"' ~/.config/nat-traversal/server.toml | head -1 | tr -d '"')
sed -i "s/token = .*/token = \"$SERVER_TOKEN\"/" ~/.config/nat-traversal/client.toml
```

### 性能问题

#### 1. 连接缓慢

**排查步骤：**
```bash
# 检查系统资源
htop
iotop
free -h
df -h

# 检查网络延迟
ping server-ip
mtr server-ip

# 检查 TCP 连接状态
ss -tnp | grep 7000 | wc -l
```

**解决方法：**
```bash
# 调整系统限制
echo "* soft nofile 65535" | sudo tee -a /etc/security/limits.conf
echo "* hard nofile 65535" | sudo tee -a /etc/security/limits.conf

# 调整内核参数
echo "net.core.somaxconn = 65535" | sudo tee -a /etc/sysctl.conf
echo "net.core.netdev_max_backlog = 5000" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

#### 2. 内存泄漏

**排查步骤：**
```bash
# 监控内存使用
watch -n 1 'ps aux | grep nat-server'
valgrind --tool=memcheck ./nat-server

# 查看详细内存信息
cat /proc/$(pidof nat-server)/status
pmap $(pidof nat-server)
```

### 日志分析

#### 启用调试日志
```bash
# 环境变量方式
RUST_LOG=debug ./nat-server
RUST_LOG=debug ./nat-client --no-gui

# 配置文件方式
sed -i 's/level = "info"/level = "debug"/' ~/.config/nat-traversal/server.toml
```

#### 常用日志过滤
```bash
# 过滤错误日志
grep ERROR /var/log/nat-traversal/server.log

# 过滤连接日志
grep "Client.*connected\|Client.*disconnected" /var/log/nat-traversal/server.log

# 过滤特定客户端日志
grep "client-id-123" /var/log/nat-traversal/server.log

# 实时监控日志
tail -f /var/log/nat-traversal/server.log | grep -E "(ERROR|WARN|Client)"
```

## 性能优化

### 服务器优化

#### 1. 系统参数调优
```bash
# 编辑 /etc/sysctl.conf
sudo tee -a /etc/sysctl.conf << EOF
# NAT Traversal 优化参数
net.core.somaxconn = 65535
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.tcp_fin_timeout = 30
net.ipv4.tcp_keepalive_time = 120
net.ipv4.tcp_keepalive_probes = 3
net.ipv4.tcp_keepalive_intvl = 15
net.ipv4.ip_local_port_range = 10000 65535
EOF

sudo sysctl -p
```

#### 2. 文件描述符限制
```bash
# 编辑 /etc/security/limits.conf
sudo tee -a /etc/security/limits.conf << EOF
nat-server soft nofile 65535
nat-server hard nofile 65535
root       soft nofile 65535
root       hard nofile 65535
EOF

# 检查当前限制
ulimit -n
cat /proc/sys/fs/file-max
```

#### 3. 应用配置优化
```toml
# server.toml 优化配置
[network]
bind_addr = "0.0.0.0"
port = 7000
max_connections = 10000

[limits]
max_tunnels_per_client = 50
max_connections_per_tunnel = 500
connection_timeout_secs = 180

[logging]
level = "warn"  # 生产环境降低日志级别
```

### 客户端优化

#### 1. 重连策略优化
```toml
# client.toml
[server]
auto_reconnect = true
reconnect_interval_secs = 15  # 缩短重连间隔
```

#### 2. GUI 性能优化
```toml
# client.toml
[gui]
enabled = true
start_minimized = true
system_tray = true
theme = "light"  # 浅色主题可能更快
```

### 监控和告警

#### 1. 系统监控脚本
```bash
#!/bin/bash
# monitor-nat-server.sh

LOG_FILE="/var/log/nat-traversal/monitor.log"
PID_FILE="/var/run/nat-server.pid"

# 检查进程是否运行
if ! pgrep -f nat-server > /dev/null; then
    echo "$(date): NAT server is not running, restarting..." >> $LOG_FILE
    systemctl restart nat-server
    exit 1
fi

# 检查端口是否监听
if ! ss -tlnp | grep -q ":7000 "; then
    echo "$(date): NAT server port 7000 not listening, restarting..." >> $LOG_FILE
    systemctl restart nat-server
    exit 1
fi

# 检查内存使用
MEM_USAGE=$(ps -o pid,vsz,rss,comm -p $(pgrep -f nat-server) | tail -1 | awk '{print $3}')
if [ $MEM_USAGE -gt 1048576 ]; then  # 1GB
    echo "$(date): High memory usage: ${MEM_USAGE}KB, restarting..." >> $LOG_FILE
    systemctl restart nat-server
fi

echo "$(date): NAT server health check passed" >> $LOG_FILE
```

#### 2. 添加到 crontab
```bash
# 每分钟检查一次
echo "* * * * * /opt/nat-traversal/monitor-nat-server.sh" | sudo crontab -
```

---

## 总结

本部署指南涵盖了 NAT 穿透工具的各种部署场景：

1. **快速开始**: 适合开发测试
2. **WSL + Windows**: 适合混合开发环境
3. **Linux 服务器**: 适合生产环境
4. **Windows 服务器**: 适合 Windows 环境

关键要点：
- TLS 证书配置是成功部署的关键
- 网络和防火墙配置不能忽视
- 监控和日志对维护很重要
- 性能优化可以显著提升用户体验

如果遇到问题，请参考故障排除章节或提交 Issue。
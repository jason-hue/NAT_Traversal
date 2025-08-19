# 快速开始指南

> 🐱 按照本指南操作，确保小猫们的安全！

## WSL + Windows 一键部署

### 1. 环境检查
```bash
# 确保在 WSL 中运行
./verify-deployment.sh
```

### 2. 自动部署
```bash  
# 一键部署所有组件
./wsl-deploy.sh
```

### 3. 启动服务
```bash
# WSL 中启动服务器
./target/release/nat-server
```

### 4. Windows 客户端
双击运行：`C:\NAT-Traversal\start-gui.bat`

## 手动部署

### 简化版本 (5分钟部署)

```bash
# 1. 获取 WSL IP
WSL_IP=$(hostname -I | awk '{print $1}')

# 2. 构建项目
cargo build --release
cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release

# 3. 生成配置
cargo run --bin nat-server -- --generate-config
cargo run --bin nat-client -- --generate-config

# 4. 生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/ST=State/L=City/O=NAT/CN=localhost" \
  -addext "subjectAltName=IP:127.0.0.1,IP:$WSL_IP"

# 5. 更新配置
sed -i "s/bind_addr = .*/bind_addr = \"$WSL_IP\"/" ~/.config/nat-traversal/server.toml
sed -i "s/addr = .*/addr = \"$WSL_IP\"/" ~/.config/nat-traversal/client.toml  
sed -i 's/tls_verify = true/tls_verify = false/' ~/.config/nat-traversal/client.toml

# 6. 部署到 Windows
mkdir -p /mnt/c/NAT-Traversal
cp ./target/x86_64-pc-windows-gnu/release/nat-client.exe /mnt/c/NAT-Traversal/
cp ~/.config/nat-traversal/client.toml /mnt/c/NAT-Traversal/

# 7. 测试连接
echo "启动服务器: ./target/release/nat-server"
echo "Windows 运行: C:\\NAT-Traversal\\nat-client.exe"
```

## 故障排除

### 连接失败？
```bash
# 检查服务器状态
ss -tlnp | grep 7000

# 检查 WSL IP
ip addr show eth0 | grep inet

# 测试连接
telnet $WSL_IP 7000
```

### TLS 错误？
```bash
# 确认证书验证已禁用
grep tls_verify ~/.config/nat-traversal/client.toml

# 重新生成证书
openssl genrsa -out server.key 2048
openssl req -new -x509 -key server.key -out server.crt -days 365 \
  -subj "/C=US/O=NAT/CN=localhost"
```

### 需要帮助？
- 查看详细文档：`README.md`
- 部署指南：`DEPLOYMENT.md` 
- 开发指南：`CLAUDE.md`
- 测试文档：`TESTING.md`

---
**部署成功后，小猫们就安全了！** 🐱✨
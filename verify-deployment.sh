#!/bin/bash
# verify-deployment.sh - 验证部署文档的完整性和准确性

set -e

echo "=== NAT Traversal 部署验证脚本 ==="
echo "本脚本将验证文档中的部署步骤是否准确"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查函数
check_command() {
    if command -v $1 >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $1 available"
        return 0
    else
        echo -e "${RED}✗${NC} $1 not found"
        return 1
    fi
}

check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} File exists: $1"
        return 0
    else
        echo -e "${RED}✗${NC} File missing: $1"
        return 1
    fi
}

check_dir() {
    if [ -d "$1" ]; then
        echo -e "${GREEN}✓${NC} Directory exists: $1"
        return 0
    else
        echo -e "${RED}✗${NC} Directory missing: $1"
        return 1
    fi
}

# 检查系统依赖
echo "1. 检查系统依赖..."
check_command "cargo" || { echo "请安装 Rust: https://rustup.rs/"; exit 1; }
check_command "openssl" || { echo "请安装 OpenSSL"; exit 1; }

# 检查编译工具（可选）
echo ""
echo "2. 检查编译工具..."
if check_command "gcc"; then
    echo -e "${GREEN}✓${NC} Native compilation available"
fi

if check_command "x86_64-w64-mingw32-gcc"; then
    echo -e "${GREEN}✓${NC} Windows cross-compilation available"
    rustup target list --installed | grep -q "x86_64-pc-windows-gnu" && \
        echo -e "${GREEN}✓${NC} Windows target installed" || \
        echo -e "${YELLOW}!${NC} Windows target not installed (rustup target add x86_64-pc-windows-gnu)"
else
    echo -e "${YELLOW}!${NC} Windows cross-compilation not available"
    echo "  安装: sudo apt install gcc-mingw-w64-x86-64"
fi

# 检查项目结构
echo ""
echo "3. 检查项目结构..."
check_dir "common" || exit 1
check_dir "server" || exit 1  
check_dir "client" || exit 1
check_dir "platform" || exit 1
check_file "Cargo.toml" || exit 1
check_file "CLAUDE.md" || exit 1
check_file "README.md" || exit 1
check_file "DEPLOYMENT.md" || exit 1

# 检查编译
echo ""
echo "4. 验证编译过程..."

echo "编译 server..."
if cargo build --bin nat-server --release >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Server compiled successfully"
    check_file "target/release/nat-server" || exit 1
else
    echo -e "${RED}✗${NC} Server compilation failed"
    echo "运行 'cargo build --bin nat-server --release' 查看详细错误"
    exit 1
fi

echo "编译 client..."
if cargo build --bin nat-client --release >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Client compiled successfully"
    check_file "target/release/nat-client" || exit 1
else
    echo -e "${RED}✗${NC} Client compilation failed"
    echo "可能缺少 GUI 依赖，尝试: cargo build -p nat-traversal-client --no-default-features --release"
fi

# 检查 Windows 交叉编译
if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "交叉编译 Windows client..."
    if cargo build --target x86_64-pc-windows-gnu -p nat-traversal-client --release >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Windows client compiled successfully"
        check_file "target/x86_64-pc-windows-gnu/release/nat-client.exe" || exit 1
    else
        echo -e "${RED}✗${NC} Windows client compilation failed"
    fi
fi

# 检查配置生成
echo ""
echo "5. 验证配置生成..."

echo "生成 server 配置..."
if ./target/release/nat-server --generate-config >/dev/null 2>&1; then
    CONFIG_DIR=$(find ~/.config -name "nat-traversal" 2>/dev/null | head -1)
    if check_file "$CONFIG_DIR/server.toml"; then
        echo -e "${GREEN}✓${NC} Server config generated"
    fi
else
    echo -e "${RED}✗${NC} Server config generation failed"
fi

echo "生成 client 配置..."
if ./target/release/nat-client --generate-config >/dev/null 2>&1; then
    CONFIG_DIR=$(find ~/.config -name "nat-traversal" 2>/dev/null | head -1)
    if check_file "$CONFIG_DIR/client.toml"; then
        echo -e "${GREEN}✓${NC} Client config generated"
    fi
else
    echo -e "${RED}✗${NC} Client config generation failed"
fi

# 检查证书生成
echo ""
echo "6. 验证证书生成..."
if openssl genrsa -out test-server.key 2048 >/dev/null 2>&1; then
    if openssl req -new -x509 -key test-server.key -out test-server.crt -days 1 \
       -subj "/C=US/ST=Test/L=Test/O=Test/CN=localhost" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Certificate generation works"
        rm -f test-server.key test-server.crt
    else
        echo -e "${RED}✗${NC} Certificate generation failed"
    fi
else
    echo -e "${RED}✗${NC} Private key generation failed"
fi

# WSL 环境检查
echo ""
echo "7. WSL 环境检查..."
if grep -q microsoft /proc/version 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Running in WSL environment"
    
    WSL_IP=$(ip addr show eth0 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
    if [ -n "$WSL_IP" ]; then
        echo -e "${GREEN}✓${NC} WSL IP detected: $WSL_IP"
        
        # 检查 Windows 路径访问
        if [ -d "/mnt/c" ]; then
            echo -e "${GREEN}✓${NC} Windows filesystem accessible at /mnt/c"
        else
            echo -e "${YELLOW}!${NC} Windows filesystem not accessible"
        fi
    else
        echo -e "${RED}✗${NC} Could not detect WSL IP"
    fi
else
    echo -e "${YELLOW}!${NC} Not running in WSL environment"
fi

# 网络检查
echo ""
echo "8. 网络配置检查..."

# 检查端口 7000 是否可用
if ! ss -tlnp | grep -q ":7000 "; then
    echo -e "${GREEN}✓${NC} Port 7000 available"
else
    echo -e "${YELLOW}!${NC} Port 7000 already in use"
    ss -tlnp | grep ":7000 "
fi

# 防火墙检查
if command -v ufw >/dev/null 2>&1; then
    UFW_STATUS=$(sudo ufw status 2>/dev/null | head -1)
    echo -e "${GREEN}✓${NC} UFW firewall: $UFW_STATUS"
fi

# 总结
echo ""
echo "=== 验证完成 ==="
echo "如果所有检查都显示 ✓，则表示环境准备就绪"
echo "如果有 ✗ 标记，请根据提示解决问题"
echo "如果有 ! 标记，表示可选功能，不影响基本使用"
echo ""
echo "下一步："
echo "1. 运行 './target/release/nat-server' 启动服务器"
echo "2. 运行 './target/release/nat-client --no-gui' 测试连接"
echo "3. 参考 DEPLOYMENT.md 进行生产环境部署"
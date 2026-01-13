# Claude Sync - 安装和部署指南

## 目录

- [服务器部署](#服务器部署)
  - [Docker 部署（推荐）](#docker-部署推荐)
  - [手动部署](#手动部署)
- [客户端安装](#客户端安装)
  - [Windows](#windows)
  - [Linux](#linux)
  - [macOS](#macos)
  - [从源码编译](#从源码编译)
- [常见问题](#常见问题)

## 服务器部署

### Docker 部署（推荐）

#### 前置要求

- Docker 20.10+
- Docker Compose 2.0+
- 至少 2GB 内存
- 10GB 可用磁盘空间

#### 安装步骤

1. **克隆仓库**

```bash
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync/docker
```

2. **配置环境变量**

```bash
cp .env.example .env
nano .env  # 或使用其他编辑器
```

必须修改以下配置项：

```bash
# 数据库密码
POSTGRES_PASSWORD=your_secure_password_here

# Redis 密码
REDIS_PASSWORD=your_secure_password_here

# MinIO 密码
MINIO_ROOT_PASSWORD=your_secure_password_here

# JWT 密钥（至少 32 字符）
JWT_SECRET=your_jwt_secret_key_minimum_32_characters_long
```

3. **启动服务**

```bash
docker-compose up -d
```

4. **检查服务状态**

```bash
docker-compose ps
```

所有服务应该显示为 `Up` 或 `healthy`：

```
NAME                    STATUS
claude-sync-db          Up (healthy)
claude-sync-redis       Up (healthy)
claude-sync-storage     Up (healthy)
claude-sync-api         Up (healthy)
```

5. **查看日志**

```bash
# 查看所有服务日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f api-server
docker-compose logs -f postgres
```

6. **停止服务**

```bash
docker-compose down
```

7. **完全清理（包括数据）**

```bash
docker-compose down -v
```

#### 生产环境建议

1. **使用反向代理**

配置 Nginx 或 Traefik 作为反向代理，提供 HTTPS。

2. **定期备份**

```bash
# 备份 PostgreSQL
docker-compose exec postgres pg_dump -U claude_sync claude_sync > backup.sql

# 备份 MinIO 数据
docker exec claude-sync-storage mc mirror /data /backup/minio
```

3. **监控**

- 使用 Prometheus + Grafana 监控服务状态
- 设置告警规则

4. **日志管理**

- 配置日志轮转
- 使用集中式日志系统（如 ELK Stack）

### 手动部署

#### 前置要求

- PostgreSQL 15+
- Redis 7+
- MinIO 或 S3 兼容存储
- Rust 1.75+（编译服务器）

#### 安装步骤

1. **安装依赖**

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install postgresql redis-server nginx
```

**macOS:**
```bash
brew install postgresql redis nginx
```

2. **配置数据库**

```bash
# 创建数据库和用户
sudo -u postgres psql
CREATE DATABASE claude_sync;
CREATE USER claude_sync WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE claude_sync TO claude_sync;
\q

# 导入初始 schema
psql -U claude_sync -d claude_sync -f migrations/init.sql
```

3. **配置 Redis**

编辑 `/etc/redis/redis.conf`:

```conf
requirepass your_redis_password
appendonly yes
```

重启 Redis:
```bash
sudo systemctl restart redis
```

4. **配置 MinIO**

```bash
# 下载 MinIO
wget https://dl.min.io/server/minio/release/linux-amd64/minio
chmod +x minio
sudo mv minio /usr/local/bin/

# 创建数据目录
sudo mkdir -p /data/minio

# 启动 MinIO
minio server /data/minio --console-address ":9001"
```

5. **编译并运行服务器**

```bash
cd server
cargo build --release

# 配置环境变量
export DATABASE_URL="postgresql://claude_sync:password@localhost/claude_sync"
export REDIS_URL="redis://:password@localhost:6379"
export MINIO_ENDPOINT="localhost:9000"
export MINIO_ACCESS_KEY="your_access_key"
export MINIO_SECRET_KEY="your_secret_key"
export JWT_SECRET="your_jwt_secret"

# 运行服务器
./target/release/claude-sync-server
```

## 客户端安装

### Windows

#### 方法 1: 下载预编译版本（推荐）

1. **下载**

访问 [Releases](https://github.com/your-repo/claude-sync/releases) 下载 `claude-sync-win.exe`

2. **安装**

将 `claude-sync-win.exe` 重命名为 `claude-sync.exe` 并移动到 PATH 中的目录：

```powershell
# 创建安装目录
mkdir C:\Tools\claude-sync

# 移动文件
move claude-sync.exe C:\Tools\claude-sync\

# 添加到 PATH（系统属性 -> 高级 -> 环境变量）
# 或使用 PowerShell 命令
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools\claude-sync", "User")
```

3. **验证安装**

```powershell
claude-sync --version
```

#### 方法 2: 使用 Chocolatey

```powershell
choco install claude-sync
```

#### 方法 3: 从源码编译

```powershell
# 安装 Rust
# 访问 https://rustup.rs/ 下载 rustup-init.exe

# 克隆仓库
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync\client

# 编译
cargo build --release

# 二进制文件位于 target\release\claude-sync.exe
```

### Linux

#### 方法 1: 下载预编译版本（推荐）

```bash
# 下载
wget https://github.com/your-repo/claude-sync/releases/latest/download/claude-sync-linux-amd64
chmod +x claude-sync-linux-amd64

# 安装
sudo mv claude-sync-linux-amd64 /usr/local/bin/claude-sync

# 验证
claude-sync --version
```

#### 方法 2: 使用包管理器

**Ubuntu/Debian:**
```bash
# 添加 APT 仓库（TODO）
sudo apt-get update
sudo apt-get install claude-sync
```

**Fedora/RHEL:**
```bash
# 添加 DNF 仓库（TODO）
sudo dnf install claude-sync
```

**Arch Linux:**
```bash
# 使用 AUR（TODO）
yay -S claude-sync
```

#### 方法 3: 从源码编译

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 克隆并编译
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync/client
cargo build --release

# 安装
sudo cp target/release/claude-sync /usr/local/bin/
```

### macOS

#### 方法 1: 使用 Homebrew（推荐）

```bash
# 添加 tap
brew tap claude-sync/tap

# 安装
brew install claude-sync

# 验证
claude-sync --version
```

#### 方法 2: 下载预编译版本

```bash
# 下载
wget https://github.com/your-repo/claude-sync/releases/latest/download/claude-sync-macos-amd64
chmod +x claude-sync-macos-amd64

# 安装
sudo mv claude-sync-macos-amd64 /usr/local/bin/claude-sync

# 验证
claude-sync --version
```

#### 方法 3: 从源码编译

```bash
# 安装 Rust
brew install rust

# 克隆并编译
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync/client
cargo build --release

# 安装
sudo cp target/release/claude-sync /usr/local/bin/
```

### 从源码编译

适用于所有平台。

#### 前置要求

- Rust 1.75+
- Protocol Buffers 编译器 (`protoc`)
- Git

#### 编译步骤

1. **安装 Rust**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. **安装 protoc**

**Ubuntu/Debian:**
```bash
sudo apt-get install protobuf-compiler
```

**macOS:**
```bash
brew install protobuf
```

**Windows:**
```powershell
choco install protoc
```

3. **克隆仓库**

```bash
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync
```

4. **编译 Protocol Buffers**

```bash
cd proto
./build.sh  # Linux/macOS
# 或
build.bat   # Windows
```

5. **编译客户端**

```bash
cd ../client
cargo build --release
```

6. **安装**

```bash
# Linux/macOS
sudo cp target/release/claude-sync /usr/local/bin/

# Windows
# target\release\claude-sync.exe 已在当前目录
```

## 初次使用

### 1. 初始化配置

```bash
claude-sync config init
```

这会在 `~/.claude-sync/config.toml` 创建配置文件。

### 2. 编辑配置

```bash
# Linux/macOS
nano ~/.claude-sync/config.toml

# Windows
notepad %USERPROFILE%\.claude-sync\config.toml
```

修改服务器地址：

```toml
[server]
endpoint = "https://your-server.com:50051"  # 修改为实际地址
```

### 3. 登录

```bash
claude-sync login
```

输入：
- Email 邮箱地址
- Password 密码

### 4. 开始同步

```bash
claude-sync sync
```

文件监控将自动启动，文件变更会实时同步。

## 常见问题

### Q: 如何查看同步状态？

A: 同步日志位于 `~/.claude-sync/sync.log`:

```bash
# Linux/macOS
tail -f ~/.claude-sync/sync.log

# Windows
Get-Content $env:USERPROFILE\.claude-sync\sync.log -Wait
```

### Q: 如何处理冲突？

A: 冲突文件会备份到 `~/.claude-sync/conflicts/`：

```bash
# 查看冲突
ls ~/.claude-sync/conflicts/

# 手动解决后，重新同步
claude-sync sync
```

### Q: 同步占用带宽太大怎么办？

A: 修改配置限制并发数：

```toml
[sync]
max_concurrent_uploads = 2
max_concurrent_downloads = 5
```

### Q: 如何只同步特定目录？

A: 使用同步规则：

```bash
# 只同步 agents
claude-sync rules add --name "only-agents" --type include --pattern "agents/**/*"
claude-sync rules add --name "exclude-all" --type exclude --pattern "**/*"
```

### Q: 服务器连接失败？

A: 检查：

1. 服务器地址是否正确
2. 网络连接是否正常
3. 防火墙是否允许连接
4. 服务器是否正在运行

```bash
# 测试连接
telnet your-server.com 50051
```

### Q: 如何卸载？

**Linux/macOS:**
```bash
sudo rm /usr/local/bin/claude-sync
rm -rf ~/.claude-sync
```

**Windows:**
```powershell
del C:\Tools\claude-sync\claude-sync.exe
Remove-Item -Recurse $env:USERPROFILE\.claude-sync
```

### Q: Token 过期怎么办？

A: Token 会自动刷新，如果刷新失败，重新登录：

```bash
claude-sync login
```

### Q: 如何同步多个账户？

A: 使用配置文件：

```bash
claude-sync sync --config ~/.claude-sync/work.toml
claude-sync sync --config ~/.claude-sync/personal.toml
```

## 更新

### 客户端更新

```bash
# 下载新版本
wget https://github.com/your-repo/claude-sync/releases/latest/download/claude-sync-linux-amd64
chmod +x claude-sync-linux-amd64
sudo mv claude-sync-linux-amd64 /usr/local/bin/claude-sync
```

### 服务器更新

```bash
cd docker
git pull
docker-compose down
docker-compose pull
docker-compose up -d
```

## 支持与帮助

- 文档：[docs/](docs/)
- 问题反馈：[GitHub Issues](https://github.com/your-repo/claude-sync/issues)
- 邮件：support@claude-sync.local

---

<p align="center">
  <sub>如有问题，请查阅文档或提交 Issue</sub>
</p>

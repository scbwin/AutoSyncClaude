# Claude Sync

<div align="center">

**🔄 跨平台 Claude CLI 配置文件同步工具**

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

</div>

## ✨ 特性

- 🔄 **实时同步** - 文件变化后 2 秒内自动同步
- 🧠 **智能合并** - 文本文件三方合并，JSON/YAML 结构化合并
- 🎯 **灵活配置** - 支持按类别、文件、设备、通配符选择同步
- 🔒 **安全可靠** - JWT Token 认证 + TLS 传输加密
- 🚀 **高性能** - Rust + gRPC，快速高效
- 🐳 **易于部署** - Docker Compose 一键启动
- 🖥️ **跨平台** - 支持 Windows、Linux、macOS

## 📋 目录

- [快速开始](#快速开始)
- [服务器部署](#服务器部署)
- [客户端安装](#客户端安装)
- [配置说明](#配置说明)
- [使用指南](#使用指南)
- [开发文档](#开发文档)
- [贡献指南](#贡献指南)

## 🚀 快速开始

### 前置要求

- **服务器端**：
  - Docker 和 Docker Compose
  - 至少 2GB 内存
  - 10GB 磁盘空间

- **客户端**：
  - Rust 1.75+ (如果从源码编译)
  - 或下载预编译二进制文件

### 30 秒快速部署

```bash
# 1. 克隆仓库
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync

# 2. 配置环境变量
cd docker
cp .env.example .env
# 编辑 .env 文件，修改密码和密钥

# 3. 启动服务器
docker-compose up -d

# 4. 等待服务启动
docker-compose ps

# 5. 安装客户端
# 从 https://releases/claude-sync 下载对应平台的二进制文件
```

## 🖥️ 服务器部署

### 使用 Docker Compose（推荐）

```bash
# 1. 配置环境变量
cp docker/.env.example docker/.env
nano docker/.env  # 修改密码和密钥

# 2. 启动所有服务
cd docker
docker-compose up -d

# 3. 查看日志
docker-compose logs -f

# 4. 停止服务
docker-compose down

# 5. 停止并删除数据
docker-compose down -v
```

### 手动部署

参见 [部署指南](docs/deployment.md)

## 💻 客户端安装

### Windows

```powershell
# 下载预编译版本
wget https://releases/claude-sync/claude-sync-win.exe -O claude-sync.exe

# 或使用 PowerShell
Invoke-WebRequest -Uri "https://releases/claude-sync/claude-sync-win.exe" -OutFile "claude-sync.exe"

# 添加到 PATH 或移动到系统目录
move claude-sync.exe C:\Windows\System32\

# 初始化配置
claude-sync.exe config init

# 登录
claude-sync.exe login
```

### Linux

```bash
# 下载预编译版本
wget https://releases/claude-sync/claude-sync-linux-amd64
chmod +x claude-sync-linux-amd64
sudo mv claude-sync-linux-amd64 /usr/local/bin/claude-sync

# 初始化配置
claude-sync config init

# 登录
claude-sync login
```

### macOS

```bash
# 使用 Homebrew 安装
brew tap claude-sync/tap
brew install claude-sync

# 或手动安装
wget https://releases/claude-sync/claude-sync-macos-amd64
chmod +x claude-sync-macos-amd64
sudo mv claude-sync-macos-amd64 /usr/local/bin/claude-sync

# 初始化配置
claude-sync config init

# 登录
claude-sync login
```

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync/client

# 编译
cargo build --release

# 二进制文件位于 target/release/claude-sync
```

## ⚙️ 配置说明

### 客户端配置文件

配置文件位置：`~/.claude-sync/config.toml`

```toml
# 服务器配置
[server]
endpoint = "https://your-server.com:50051"
timeout = 30

# 认证配置
[auth]
token = "your-access-token"
device_id = "device-uuid"
device_name = "My Windows PC"

# 同步配置
[sync]
interval = 60  # 同步间隔（秒）
batch_window = 2000  # 批处理窗口（毫秒）
max_concurrent_uploads = 5
max_concurrent_downloads = 10
sync_on_startup = true
sync_on_shutdown = true
claude_dir = "~/.claude"  # Claude CLI 配置目录

# 选择性同步规则
[[sync.rules]]
name = "include-agents"
type = "include"
pattern = "agents/**/*"
file_type = "agent"
priority = 100

[[sync.rules]]
name = "exclude-cache"
type = "exclude"
pattern = "cache/**/*"
priority = 1000

# 冲突解决策略
[conflict]
strategy = "prompt"  # 'local', 'remote', 'auto', 'prompt'
text_merge = true
json_merge = true
backup_dir = "~/.claude-sync/conflicts"

# 性能优化
[performance]
debounce_delay = 500  # 文件监控防抖（毫秒）
large_file_threshold = 10  # 大文件阈值（MB）
enable_compression = true
max_retries = 3
retry_delay = 5

# 日志配置
[logging]
level = "info"  # 'debug', 'info', 'warn', 'error'
file = "~/.claude-sync/sync.log"
max_size = 10  # MB
max_backups = 3
```

### 同步规则说明

- **按类别同步**：`file_type` 可选值
  - `agent` - agents/ 目录
  - `skill` - skills/ 目录
  - `plugin` - plugins/ 目录
  - `command` - commands/ 目录
  - `config` - 配置文件
  - `plan` - plans/ 目录

- **模式匹配**：
  - Glob 模式：`agents/**/*.md`
  - 精确路径：`agents/my-agent.md`
  - 通配符：`*.json`

- **优先级**：数字越大优先级越高，规则按优先级从高到低匹配

## 📖 使用指南

### 基本命令

```bash
# 初始化配置
claude-sync config init

# 登录
claude-sync login

# 开始同步
claude-sync sync

# 查看设备列表
claude-sync list-devices

# 管理同步规则
claude-sync rules list
claude-sync rules add --name "include-skills" --type include --pattern "skills/**/*"
claude-sync rules remove <rule-id>

# 登出
claude-sync logout
```

### 同步工作流

1. **初始化**：首次使用运行 `config init` 生成配置文件
2. **登录**：运行 `login` 输入邮箱和密码，获取访问令牌
3. **配置规则**（可选）：使用 `rules` 命令配置选择性同步
4. **开始同步**：运行 `sync` 启动文件监控和自动同步
5. **完成**：文件变更会自动同步到其他设备

### 选择性同步示例

```bash
# 只同步 agents 和 skills
claude-sync rules add --name "only-agents" --type include --pattern "agents/**/*"
claude-sync rules add --name "only-skills" --type include --pattern "skills/**/*"
claude-sync rules add --name "exclude-others" --type exclude --pattern "**/*"

# 排除缓存目录
claude-sync rules add --name "no-cache" --type exclude --pattern "cache/**/*"
claude-sync rules add --name "no-downloads" --type exclude --pattern "downloads/**/*"

# 按文件类型
claude-sync rules add --name "json-only" --type include --pattern "*.json" --file-type config
```

## 🔧 开发文档

### 项目结构

```
claude-sync/
├── server/          # 服务器端代码
│   ├── src/
│   │   ├── main.rs
│   │   ├── auth.rs
│   │   ├── db.rs
│   │   ├── storage.rs
│   │   └── grpc/
│   └── Cargo.toml
├── client/          # 客户端代码
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── watcher.rs
│   │   ├── sync.rs
│   │   └── ...
│   └── Cargo.toml
├── proto/           # Protocol Buffers 定义
│   └── sync.proto
├── docker/          # Docker 配置
│   ├── docker-compose.yml
│   └── .env.example
├── migrations/      # 数据库迁移
│   └── init.sql
└── docs/            # 文档
```

### 开发环境设置

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Protocol Buffers 编译器
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# Windows (使用 Chocolatey)
choco install protoc

# 克隆仓库
git clone https://github.com/your-repo/claude-sync.git
cd claude-sync

# 编译 Protocol Buffers
cd proto
./build.sh  # Linux/macOS
# 或
build.bat   # Windows

# 编译服务器
cd ../server
cargo build

# 编译客户端
cd ../client
cargo build
```

### 运行测试

```bash
# 服务器端测试
cd server
cargo test

# 客户端测试
cd client
cargo test

# 集成测试
cd ..
./scripts/integration-test.sh
```

## 🤝 贡献指南

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

### 开发路线图

- [x] 基础同步功能
- [x] 文件监控
- [x] 冲突检测和解决
- [x] 选择性同步
- [ ] Web UI
- [ ] 端到端加密
- [ ] 移动端应用
- [ ] 团队协作功能

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [tonic](https://github.com/hyperium/tonic) - gRPC Rust 框架
- [notify](https://github.com/notify-rs/notify) - 文件系统监控
- [SQLx](https://github.com/launchbadge/sqlx) - 异步 SQL 工具包
- [MinIO](https://min.io/) - 对象存储

## 📮 联系方式

- 问题反馈：[GitHub Issues](https://github.com/your-repo/claude-sync/issues)
- 邮件：support@claude-sync.local
- 文档：[docs/](docs/)

---

<p align="center">
  <sub>Built with ❤️ by the Claude Sync Team</sub>
</p>

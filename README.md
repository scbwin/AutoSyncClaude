# Claude Sync

<div align="center">

**🔄 跨平台 Claude CLI 配置文件同步工具**

[![Build Status](https://github.com/scbwin/AutoSyncClaude/workflows/Build%20and%20Test/badge.svg)](https://github.com/scbwin/AutoSyncClaude/actions)
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
- 🎨 **GUI 客户端** - 基于 Tauri 的桌面应用程序

## 📋 目录

- [快速开始](#快速开始)
- [服务器部署](#服务器部署)
- [客户端安装](#客户端安装)
- [GUI 客户端](#gui-客户端)
- [配置说明](#配置说明)
- [使用指南](#使用指南)
- [开发文档](#开发文档)
- [构建指南](#构建指南)
- [贡献指南](#贡献指南)

## 🚀 快速开始

### 前置要求

- **服务器端**：
  - Docker 和 Docker Compose
  - 至少 2GB 内存
  - 10GB 磁盘空间

- **命令行客户端**：
  - Rust 1.75+ (如果从源码编译)
  - 或下载预编译二进制文件

- **GUI 客户端**：
  - Windows 10/11, macOS 10.15+, 或 Ubuntu 20.04+
  - 无需额外依赖

### 30 秒快速部署

```bash
# 1. 克隆仓库
git clone https://github.com/scbwin/AutoSyncClaude.git
cd AutoSyncClaude

# 2. 配置环境变量
cd docker
cp .env.example .env
# 编辑 .env 文件，修改密码和密钥

# 3. 启动服务器
docker-compose up -d

# 4. 等待服务启动
docker-compose ps

# 5. 安装客户端
# 从 GitHub Releases 下载对应平台的二进制文件
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
wget https://github.com/scbwin/AutoSyncClaude/releases/download/v0.1.0/claude-sync-windows.exe -O claude-sync.exe

# 或使用 PowerShell
Invoke-WebRequest -Uri "https://github.com/scbwin/AutoSyncClaude/releases/download/v0.1.0/claude-sync-windows.exe" -OutFile "claude-sync.exe"

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
wget https://github.com/scbwin/AutoSyncClaude/releases/download/v0.1.0/claude-sync-linux-amd64
chmod +x claude-sync-linux-amd64
sudo mv claude-sync-linux-amd64 /usr/local/bin/claude-sync

# 初始化配置
claude-sync config init

# 登录
claude-sync login
```

### macOS

```bash
# 下载预编译版本
wget https://github.com/scbwin/AutoSyncClaude/releases/download/v0.1.0/claude-sync-macos-amd64
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
git clone https://github.com/scbwin/AutoSyncClaude.git
cd AutoSyncClaude/client

# 安装 protoc
# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf

# Windows (使用 Chocolatey)
choco install protoc

# 编译
cargo build --release

# 二进制文件位于 target/release/claude-sync-client
```

## 🖥️ GUI 客户端

GUI 客户端提供友好的图形界面，适合不熟悉命令行的用户。

### 下载安装

从 [GitHub Releases](https://github.com/scbwin/AutoSyncClaude/releases) 下载对应平台的安装包：

- **Windows**: `.msi` 或 `.exe` 安装包
- **macOS**: `.dmg` 镜像文件
- **Linux**: `.deb` 包或 `.AppImage` 文件

### 从源码构建

```bash
cd gui-client

# 安装依赖
npm install

# 开发模式运行
npm run dev

# 构建生产版本
npm run build

# 构建产物位于 src-tauri/target/release/
```

### 功能特性

- 🎨 直观的用户界面
- 📊 实时同步状态显示
- ⚙️ 图形化配置管理
- 📋 同步规则管理
- 🔍 冲突解决向导
- 📈 同步统计和日志

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

### 命令行客户端基本命令

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

### GUI 客户端使用流程

1. **启动应用** - 双击桌面图标或从应用菜单启动
2. **配置服务器** - 在设置中输入服务器地址
3. **登录账户** - 输入邮箱和密码登录
4. **配置同步** - 设置要同步的文件和规则
5. **开始同步** - 点击"开始同步"按钮
6. **查看状态** - 在主界面查看同步进度和状态

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
AutoSyncClaude/
├── server/          # 服务器端代码 (Rust + gRPC + PostgreSQL)
│   ├── src/
│   │   ├── main.rs
│   │   ├── auth.rs
│   │   ├── db.rs
│   │   ├── storage.rs
│   │   └── grpc/
│   └── Cargo.toml
├── client/          # 命令行客户端 (Rust + gRPC)
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── watcher.rs
│   │   ├── sync.rs
│   │   └── ...
│   └── Cargo.toml
├── gui-client/      # GUI 客户端 (Tauri + Web 技术)
│   ├── src/         # 前端代码
│   ├── src-tauri/   # Tauri 后端 (Rust)
│   └── package.json
├── proto/           # Protocol Buffers 定义
│   └── sync.proto
├── docker/          # Docker 配置
│   ├── docker-compose.yml
│   └── .env.example
├── .github/
│   └── workflows/   # GitHub Actions CI/CD
└── docs/            # 文档
```

### 开发环境设置

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Node.js (用于 GUI 客户端开发)
# 从 https://nodejs.org/ 下载并安装

# 安装 Protocol Buffers 编译器
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# Windows (使用 Chocolatey)
choco install protoc

# 克隆仓库
git clone https://github.com/scbwin/AutoSyncClaude.git
cd AutoSyncClaude

# 编译 Protocol Buffers
cd proto
./build.sh  # Linux/macOS
# 或
build.bat   # Windows
```

## 🏗️ 构建指南

### 构建服务器

```bash
cd server
cargo build --release

# 运行服务器
./target/release/claude-sync-server
```

### 构建命令行客户端

```bash
cd client
cargo build --release

# 运行客户端
./target/release/claude-sync-client
```

### 构建 GUI 客户端

```bash
cd gui-client

# 安装依赖
npm install

# 开发模式
npm run dev

# 生产构建
npm run build

# 构建产物位于 src-tauri/target/release/bundle/
```

### 跨平台构建

项目使用 GitHub Actions 自动构建所有平台：

- ✅ **Linux** - Ubuntu 最新版本
- ✅ **Windows** - Windows Server 2022
- ✅ **macOS** - macOS 11+ (支持 Intel 和 Apple Silicon)

构建产物包括：
- 命令行客户端二进制文件
- GUI 客户端安装包 (MSI, DMG, DEB, AppImage)

### 运行测试

```bash
# 服务器端测试
cd server
cargo test

# 客户端测试
cd client
cargo test

# 格式检查
cd server && cargo fmt -- --check
cd ../client && cargo fmt -- --check

# Clippy 检查
cd server && cargo clippy -- -D warnings
cd ../client && cargo clippy -- -D warnings
```

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 代码规范

- Rust 代码遵循 `rustfmt` 格式化
- 通过 `cargo clippy` 检查
- 添加适当的单元测试
- 更新相关文档

### 提交信息规范

使用语义化提交信息：

- `feat:` - 新功能
- `fix:` - 修复 bug
- `docs:` - 文档更新
- `style:` - 代码格式（不影响功能）
- `refactor:` - 重构
- `test:` - 测试相关
- `chore:` - 构建/工具链相关

示例：
```
feat: add conflict resolution for YAML files
fix: resolve memory leak in file watcher
docs: update installation guide for Windows
```

### 开发路线图

- [x] 基础同步功能
- [x] 文件监控和实时同步
- [x] 冲突检测和解决
- [x] 选择性同步规则
- [x] GUI 客户端
- [x] 跨平台支持
- [ ] Web UI
- [ ] 端到端加密
- [ ] 移动端应用
- [ ] 团队协作功能
- [ ] 插件系统

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [tonic](https://github.com/hyperium/tonic) - gRPC Rust 框架
- [notify](https://github.com/notify-rs/notify) - 文件系统监控
- [SQLx](https://github.com/launchbadge/sqlx) - 异步 SQL 工具包
- [Tokio](https://tokio.rs/) - 异步运行时
- [pngjs](https://github.com/lukeapage/pngjs) - PNG 图标生成

## 📮 联系方式

- 🐛 问题反馈：[GitHub Issues](https://github.com/scbwin/AutoSyncClaude/issues)
- 💬 讨论：[GitHub Discussions](https://github.com/scbwin/AutoSyncClaude/discussions)
- 📧 邮件：support@claude-sync.local
- 📚 文档：[docs/](docs/)

## 🌟 Star History

如果这个项目对你有帮助，请给我们一个 ⭐️ Star！

---

<p align="center">
  <sub>Built with ❤️ and ☕ by the Claude Sync Team</sub>
</p>

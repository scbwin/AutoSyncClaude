# Protocol Buffers 安装指南

## 1. 安装 protoc 编译器

### Windows

#### 方法 1: 使用预编译二进制文件（推荐）

1. 下载最新版本的 protoc:
   - 访问 https://github.com/protocolbuffers/protobuf/releases
   - 下载 `protoc-[version]-win64.zip` (例如 `protoc-25.0-win64.zip`)

2. 解压到本地目录，例如 `C:\protoc`

3. 将 `C:\protoc\bin` 添加到系统 PATH:
   - 右键"此电脑" → "属性" → "高级系统设置" → "环境变量"
   - 在"系统变量"中找到 `Path`
   - 点击"编辑"，添加 `C:\protoc\bin`

4. 验证安装:
   ```bash
   protoc --version
   ```
   应该显示: `libprotoc 25.0` (或类似版本)

#### 方法 2: 使用包管理器

**使用 Scoop (推荐):**
```bash
scoop install protoc
```

**使用 Chocolatey:**
```bash
choco install protoc
```

### Linux

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y protobuf-compiler

# CentOS/RHEL
sudo yum install -y protobuf-compiler

# Arch Linux
sudo pacman -S protobuf
```

### macOS

```bash
# 使用 Homebrew
brew install protobuf
```

## 2. 安装 Rust protobuf 插件

需要安装 `protoc-gen-rust` 和 `protoc-gen-tonic`:

```bash
cargo install protobuf
cargo install tonic-build
```

或者使用 `protoc-gen-rust` 和 `protoc-gen-tonic`:

```bash
cargo install protoc-gen-rust
cargo install protoc-gen-tonic
```

## 3. 验证安装

检查所有工具是否正确安装:

```bash
# 检查 protoc
protoc --version

# 检查 Rust 插件
protoc-gen-rust --version || echo "protoc-gen-rust not found"
protoc-gen-tonic --version || echo "protoc-gen-tonic not found"
```

## 4. 构建 Protocol Buffers

安装完成后，在 `proto` 目录执行:

**Windows:**
```bash
cd D:\syncClaude\proto
build.bat
```

**Linux/macOS:**
```bash
cd /path/to/syncClaude/proto
chmod +x build.sh
./build.sh
```

## 5. 常见问题

### 问题: `protoc-gen-rust: not found`

**解决方案:**
```bash
cargo install protoc-gen-rust
```

### 问题: `protoc-gen-tonic: not found`

**解决方案:**
```bash
cargo install protoc-gen-tonic
```

### 问题: `error: missing field 'session'`

这可能是由于 protoc 版本不兼容。确保使用最新版本的 protoc (建议 25.0+)。

### 问题: Windows 下找不到 `protoc`

确保 `C:\protoc\bin` (或你的安装路径) 已添加到 PATH 环境变量。

添加后需要:
1. 关闭并重新打开命令行窗口
2. 或者运行 `refreshenv` (如果使用 Scoop)

## 6. 生成的文件位置

成功构建后，生成的文件将位于:

- **服务器端**: `server/src/proto/`
  - `sync.rs` (消息定义)
  - `claude_sync.rs` (服务 trait)
  - `claude_sync.server.rs` (服务器端实现)

- **客户端**: `client/src/proto/`
  - `sync.rs` (消息定义)
  - `claude_sync.rs` (客户端 stub)
  - `claude_sync.client.rs` (客户端实现)

## 7. 更新 .proto 文件

如果修改了 `sync.proto`，只需重新运行构建脚本:

```bash
cd D:\syncClaude\proto
build.bat
```

---

**文档版本**: 1.0
**最后更新**: 2026-01-13

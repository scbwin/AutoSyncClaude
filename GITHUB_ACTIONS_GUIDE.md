# GitHub Actions 构建指南

本项目使用 GitHub Actions 进行云端构建，可以自动生成 Protocol Buffers 代码并编译项目。

## 前置条件

1. GitHub 账户
2. 将代码推送到 GitHub 仓库

## 快速开始

### 1. 创建 GitHub 仓库

```bash
# 在 GitHub 网站上创建新仓库，然后执行以下命令

cd D:\dev_syncClaude
git init
git add .
git commit -m "Initial commit"

# 添加远程仓库（替换为您的仓库地址）
git remote add origin https://github.com/YOUR_USERNAME/claude-sync.git

# 推送代码
git branch -M main
git push -u origin main
```

### 2. 触发构建

推送代码后，GitHub Actions 会自动运行构建流程。您也可以手动触发：

1. 访问您的 GitHub 仓库
2. 点击 "Actions" 标签
3. 选择 "Build and Test" workflow
4. 点击 "Run workflow" 按钮

### 3. 下载构建产物

构建完成后，您可以下载生成的二进制文件和 protobuf 代码：

1. 进入 Actions 页面
2. 点击具体的构建任务
3. 滚动到底部的 "Artifacts" 部分
4. 下载您需要的文件：
   - `claude-sync-server-linux` - 服务器端可执行文件
   - `claude-sync-client-linux` - Linux 客户端
   - `claude-sync-client-windows.exe` - Windows 客户端
   - `claude-sync-client-macos` - macOS 客户端
   - `server-proto-code` - 服务器端 protobuf 生成的代码
   - `client-proto-code` - 客户端 protobuf 生成的代码

## 本地使用生成的代码

### 方式一：下载 protobuf 代码

1. 从 GitHub Actions 下载 `server-proto-code` 和 `client-proto-code`
2. 解压到对应的 `src/proto` 目录
3. 在本地进行开发

### 方式二：直接下载编译好的二进制文件

1. 下载对应平台的二进制文件
2. 解压即可直接使用

## Workflow 说明

`.github/workflows/build.yml` 定义了以下任务：

1. **build-server** - 构建服务器端（Linux）
2. **build-client** - 构建客户端（多平台：Linux、Windows、macOS）
3. **generate-proto** - 生成 Protocol Buffers 代码
4. **format-check** - 检查代码格式
5. **clippy-check** - 运行 Clippy 代码检查

## 本地开发建议

如果您需要在本地开发，建议：

1. **创建不含中文的 Windows 用户**（推荐）
   - 用户名：`dev` 或 `developer`
   - 在该用户环境下进行开发

2. **使用 WSL2**（如果可用）
   ```bash
   # 在 WSL 中安装 Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **直接使用 GitHub Actions 构建的产物**
   - 在 GitHub 上修改代码
   - 通过 Actions 构建
   - 下载生成的文件到本地测试

## 故障排除

### 构建失败

如果 GitHub Actions 构建失败，检查：

1. `Cargo.toml` 中的依赖是否正确
2. `build.rs` 文件是否存在且正确
3. `.proto` 文件路径是否正确

### 下载的文件无法运行

1. Linux/macOS: 需要添加执行权限
   ```bash
   chmod +x claude-sync-server
   chmod +x claude-sync-client
   ```

2. Windows: 可能需要允许 Windows Defender 运行

## 相关链接

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Rust Setup Action](https://github.com/actions-rust-lang/setup-rust-toolchain)

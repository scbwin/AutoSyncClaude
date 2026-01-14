# 下载 GUI 客户端

## 快速下载步骤

### 1. 查看构建状态

访问 GitHub Actions 页面：
```
https://github.com/scbwin/AutoSyncClaude/actions
```

### 2. 找到最新的 GUI 构建任务

- 寻找名为 "Build GUI Client" 的工作流
- 点击最新的运行记录（应该显示绿色的 ✅ 或黄色的 ●）

### 3. 下载安装包

在构建页面底部找到 **"Artifacts"** 部分，下载对应你操作系统的文件：

#### Windows 用户
- 📦 `claude-sync-gui-windows` - 包含：
  - `.msi` 安装程序（推荐）
  - `.exe` 安装程序

#### Linux 用户
- 📦 `claude-sync-gui-linux` - 包含：
  - `.deb` 包（适用于 Debian/Ubuntu）
  - `.AppImage` 通用包（适用于所有发行版）

#### macOS 用户
- 📦 `claude-sync-gui-macos` - 包含：
  - `.dmg` 镜像文件
  - `.app` 应用程序

### 4. 安装运行

#### Windows
```bash
# 双击 .msi 或 .exe 文件，按提示安装
# 或从命令行运行
claude-sync-gui.exe
```

#### Linux
```bash
# DEB 包
sudo dpkg -i *.deb

# AppImage（直接运行）
chmod +x *.AppImage
./claude-sync-gui*.AppImage
```

#### macOS
```bash
# 打开 .dmg 文件
# 将应用拖拽到 Applications 文件夹
open -a "Claude Sync GUI"
```

## 构建历史

所有历史构建版本都会保存在 GitHub Actions 中，有效期 90 天。

如果你需要特定版本：
1. 进入 Actions 页面
2. 选择对应的工作流运行记录
3. 滚动到底部下载 Artifacts

## 当前构建状态

![Build Status](https://github.com/scbwin/AutoSyncClaude/actions/workflows/build.yml/badge.svg)

点击上面的徽章可以查看详细的构建日志。

## 故障排除

### 构建失败

如果看到构建失败，可能的原因：
- 代码有编译错误（检查错误日志）
- 依赖问题（等待自动重试）
- CI/CD 环境问题

### 下载失败

如果无法下载：
- 确保你已登录 GitHub
- 检查网络连接
- 尝试刷新页面后重新下载

### 安装问题

#### Windows
- 确保已安装 VC++ Redistributable
- 以管理员身份运行安装程序

#### Linux
```bash
# Ubuntu/Debian 依赖
sudo apt-get install libwebkit2gtk-4.0-dev

# Fedora
sudo dnf install webkit2gtk3-devel

# Arch Linux
sudo pacman -S webkit2gtk
```

#### macOS
- 确保系统版本 >= 10.15 (Catalina)
- 如果看到"无法验证开发者"警告：
  ```bash
  xattr -cr /Applications/Claude\ Sync\ GUI.app
  ```

## 下次更新

当你再次推送代码到 GitHub 时，会自动触发新的构建。只需：
1. 等待构建完成（约 10-15 分钟）
2. 下载最新的 Artifacts
3. 覆盖安装即可

## 开发构建

如果你想自己修改并构建 GUI 客户端，请参考 `GUI_BUILD_INSTRUCTIONS.md`。

---

需要帮助？请查看：
- [GUI_CLIENT_README.md](GUI_CLIENT_README.md) - 技术文档
- [GUI_BUILD_INSTRUCTIONS.md](GUI_BUILD_INSTRUCTIONS.md) - 构建指南
- [GitHub Issues](https://github.com/scbwin/AutoSyncClaude/issues) - 问题反馈

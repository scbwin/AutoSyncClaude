# GUI 客户端构建指南

## 方法一：在 GitHub Actions 上构建（推荐）

这是最简单的方法，可以避免本地 Windows 链接器问题。

### 步骤：

1. **推送代码到 GitHub**
   ```bash
   git push origin main
   ```

2. **触发构建**
   - 构建会在推送时自动触发
   - 或者访问 GitHub 页面手动触发：https://github.com/scbwin/AutoSyncClaude/actions

3. **下载构建产物**
   - 等待构建完成（大约 10-15 分钟）
   - 进入构建详情页面
   - 在 "Artifacts" 部分下载以下文件：
     - `claude-sync-gui-linux` - Linux 版本（.deb 和 .AppImage）
     - `claude-sync-gui-windows` - Windows 版本（.msi 和 .exe）
     - `claude-sync-gui-macos` - macOS 版本（.dmg 和 .app）

4. **安装运行**
   - **Windows**: 双击 `.msi` 或 `.exe` 文件安装
   - **Linux**: 安装 `.deb` 文件或直接运行 `.AppImage`
   - **macOS**: 打开 `.dmg` 文件并拖拽到 Applications

## 方法二：本地构建（需要修复路径问题）

如果要在本地构建，需要先解决中文路径问题。

### 选项 A：移动项目到不含中文的路径

```bash
# 将项目移动到新路径
# 例如：D:\dev_syncClaude → D:\projects\syncClaude
cd D:\projects\syncClaude\gui-client
npm install
npm run build
```

### 选项 B：使用 GNU 工具链

```bash
# 安装 GNU 工具链
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu

# 构建
cd gui-client
npm install
npm run build
```

### 选项 C：在 WSL 中构建

```bash
# 在 WSL 中
cd /mnt/d/dev_syncClaude/gui-client
npm install
npm run build
```

## GUI 客户端功能

### 主要界面

1. **仪表盘**
   - 显示同步统计信息
   - 查看最近活动
   - 连接状态指示

2. **同步控制**
   - 手动启动/停止同步
   - 选择同步模式（自动/手动/双向）
   - 实时进度显示

3. **规则管理**
   - 添加/删除同步规则
   - 支持排除/包含模式
   - 设置优先级

4. **设备管理**
   - 查看已连接设备
   - 设备状态监控

5. **设置**
   - 服务器地址配置
   - 同步间隔设置
   - 主题切换（跟随系统/浅色/深色）
   - 语言选择（中文/English）

### 使用流程

1. **首次启动**
   - 点击侧边栏底部的用户信息区域
   - 在弹出的登录对话框中输入邮箱和密码
   - 点击"登录"按钮

2. **配置服务器**
   - 进入"设置"页面
   - 填写服务器地址（例如：http://localhost:50051）
   - 设置超时时间
   - 配置 Claude 目录路径
   - 点击"保存设置"

3. **开始同步**
   - 进入"同步"页面
   - 选择同步模式
   - 点击"开始同步"按钮
   - 观察同步进度

4. **管理规则**
   - 进入"规则"页面
   - 点击"添加规则"创建新的同步规则
   - 设置规则名称、类型、匹配模式
   - 点击规则右侧的删除按钮可移除规则

## 技术细节

- **框架**: Tauri 1.6 (基于 Rust)
- **前端**: HTML5 + CSS3 + Vanilla JavaScript
- **后端**: Rust + Tokio
- **配置存储**: JSON 文件
- **跨平台**: 支持 Windows、Linux、macOS

## 故障排除

### 问题：无法连接到服务器

**解决方案**：
- 检查服务器是否运行
- 确认服务器地址配置正确
- 检查网络连接

### 问题：同步失败

**解决方案**：
- 查看错误消息
- 检查 Claude 目录路径是否正确
- 确认有足够的磁盘空间

### 问题：GUI 无法启动

**解决方案**：
- 确认已安装必要的运行库
- Windows: 安装 VC++ Redistributable
- Linux: 安装 webkit2gtk 库

## 获取帮助

如果遇到问题：
1. 查看 `GUI_CLIENT_README.md` 获取更多技术细节
2. 检查 GitHub Issues 页面
3. 查看应用日志文件（通常在配置目录）

## 开发者

如果你想修改或扩展 GUI 客户端：

1. **开发模式**
   ```bash
   cd gui-client
   npm install
   npm run dev  # 启动开发服务器（热重载）
   ```

2. **文件结构**
   - `src/` - 前端代码（HTML、CSS、JavaScript）
   - `src-tauri/src/` - Rust 后端代码
   - `src-tauri/src/commands/` - Tauri 命令处理器

3. **添加新功能**
   - 前端：修改 `src/` 目录下的文件
   - 后端：在 `src-tauri/src/commands/` 添加新的命令
   - 在 `main.rs` 中注册新命令

---

享受使用 GUI 客户端！

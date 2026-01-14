# Claude Sync GUI 客户端

## 当前状态

GUI 客户端的所有代码已经创建完成，包括：
- ✅ Tauri 项目配置
- ✅ Rust 后端 API（config、auth、sync、rules、devices）
- ✅ 前端界面（HTML、CSS、JavaScript）
- ✅ 响应式设计和现代 UI

## 编译问题

**当前存在一个 Windows 链接器问题**，导致编译失败。

### 问题原因

错误信息：`link: extra operand`

这是由于用户路径包含中文字符（`宋传波`）导致的。Windows 链接器 (`link.exe`) 在处理包含非 ASCII 字符的路径时可能出现问题。

### 解决方案

有以下几种解决方法：

#### 方案 1：使用不含中文的路径（推荐）

将项目移动到不含中文字符的路径：

```bash
# 例如，从 D:\dev_syncClaude 移动到 D:\dev\syncClaude
# 或者使用 C:\Projects\syncClaude
```

#### 方案 2：切换到 GNU 工具链

使用 MinGW-w64 工具链代替 MSVC：

```bash
# 安装 GNU 工具链
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu

# 然后重新编译
cd gui-client/src-tauri
cargo build
```

#### 方案 3：在 WSL 中编译

在 Windows Subsystem for Linux (WSL) 中编译：

```bash
# 在 WSL 中
cd /mnt/d/dev_syncClaude/gui-client/src-tauri
cargo build
```

## 项目结构

```
gui-client/
├── src/                    # 前端源代码
│   ├── index.html         # 主 HTML 文件
│   ├── styles.css         # 样式表
│   └── app.js             # JavaScript 应用逻辑
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # 应用入口
│   │   ├── config.rs      # 配置管理
│   │   ├── state.rs       # 同步状态
│   │   └── commands/      # Tauri 命令
│   │       ├── auth.rs    # 认证命令
│   │       ├── config.rs  # 配置命令
│   │       ├── devices.rs # 设备命令
│   │       ├── rules.rs   # 规则命令
│   │       └── sync.rs    # 同步命令
│   ├── Cargo.toml         # Rust 依赖
│   ├── tauri.conf.json    # Tauri 配置
│   └── build.rs           # 构建脚本
└── package.json           # Node.js 配置
```

## 功能特性

### 1. 仪表盘
- 显示同步统计（已同步文件、失败文件等）
- 最近活动列表
- 连接状态显示

### 2. 同步控制
- 手动启动/停止同步
- 选择同步模式（自动、手动、双向）
- 实时进度显示
- 文件同步列表

### 3. 规则管理
- 添加/删除同步规则
- 支持排除/包含模式
- 优先级设置

### 4. 设备管理
- 查看已连接设备
- 设备状态显示

### 5. 设置
- 服务器地址配置
- 同步间隔设置
- 主题切换（跟随系统/浅色/深色）
- 语言选择
- 托盘和通知设置

## 使用方法

解决编译问题后，可以：

### 开发模式
```bash
cd gui-client
npm install    # 安装 Tauri CLI（首次）
npm run dev    # 启动开发服务器
```

### 构建发布版本
```bash
cd gui-client
npm run build  # 构建可执行文件
```

构建后的可执行文件位于：
- Windows: `src-tauri/target/release/claude-sync-gui.exe`
- Linux: `src-tauri/target/release/claude-sync-gui`
- macOS: `src-tauri/target/release/bundle/macos/Claude Sync GUI.app`

## 技术栈

- **前端框架**: Tauri 1.6
- **UI**: HTML5 + CSS3 + Vanilla JavaScript
- **后端**: Rust + Tokio
- **配置管理**: JSON
- **样式**: 现代响应式设计，支持深色模式

## 后续开发建议

1. **集成同步逻辑**
   - 当前 TODO 注释标记了需要集成客户端代码的位置
   - 可以通过 HTTP API 与服务器通信
   - 或者修复链接器问题后重新添加 claude-sync 库依赖

2. **添加更多功能**
   - 实时文件监控
   - 冲突解决界面
   - 同步历史记录
   - 文件预览功能

3. **改进 UI**
   - 添加系统托盘图标
   - 添加桌面通知
   - 添加启动动画

4. **测试**
   - 单元测试
   - 集成测试
   - 跨平台测试（Windows、Linux、macOS）

## 许可证

MIT

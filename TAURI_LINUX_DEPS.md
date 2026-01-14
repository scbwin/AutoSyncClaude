# Tauri Linux 依赖完整列表

## 📦 必需的系统库

这是 Tauri 1.x 在 Ubuntu/Debian Linux 上构建所需的所有依赖。

### 核心依赖

```bash
# WebKitGTK 引擎（必需）
libwebkit2gtk-4.1-dev     # Web 渲染引擎

# GTK3 库（必需）
libgtk-3-dev              # GUI 工具包

# 构建工具（必需）
build-essential           # GCC, make 等编译工具

# SSL 支持（必需）
libssl-dev                # OpenSSL 开发库
```

### 功能相关依赖

```bash
# 应用指示器（系统托盘）
libayatana-appindicator3-dev

# SVG 图标支持
librsvg2-dev

# HTTP 功能（WebKit 依赖）
libsoup2.4-dev            # HTTP 客户端库
```

### 开发工具

```bash
# 协议缓冲区编译器
protobuf-compiler

# 下载工具
curl
wget

# 文件类型检测
file
```

---

## 🔧 完整安装命令

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
  protobuf-compiler \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libgtk-3-dev \
  libsoup2.4-dev
```

### Fedora

```bash
sudo dnf install \
  protobuf-compiler \
  webkit2gtk4.1-devel \
  gcc \
  gcc-c++ \
  make \
  curl \
  wget \
  file \
  openssl-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  gtk3-devel \
  libsoup-devel
```

### Arch Linux

```bash
sudo pacman -S \
  protobuf \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  libappindicator-gtk3 \
  librsvg \
  gtk3 \
  libsoup
```

---

## 📋 依赖说明

### libwebkit2gtk-4.1-dev
- **用途**: Web 渲染引擎，Tauri 的核心
- **包大小**: ~50 MB
- **为什么需要**: Tauri 使用 WebKit 来渲染 UI

### libgtk-3-dev
- **用途**: GTK+ 3.0 开发库
- **包大小**: ~5 MB
- **为什么需要**: 窗口管理、原生 UI 集成

### libsoup2.4-dev
- **用途**: HTTP 客户端库
- **包大小**: ~2 MB
- **为什么需要**: WebKit 的网络功能

### libayatana-appindicator3-dev
- **用途**: 应用指示器（系统托盘）
- **包大小**: ~1 MB
- **为什么需要**: 系统托盘图标支持

### librsvg2-dev
- **用途**: SVG 图标渲染
- **包大小**: ~1 MB
- **为什么需要**: SVG 图标支持

### build-essential
- **用途**: 编译工具链（gcc, g++, make）
- **包大小**: ~100 MB
- **为什么需要**: 编译 Rust 代码

---

## 🔍 依赖问题排查

### 问题 1: soup2-sys 编译失败

**错误信息**:
```
The system library `libsoup-2.4` required by crate `soup2-sys` was not found
```

**解决方案**:
```bash
sudo apt-get install libsoup2.4-dev
```

### 问题 2: webkit2gtk 找不到

**错误信息**:
```
Package 'webkit2gtk-4.0' not found
```

**解决方案**:
```bash
# Ubuntu 22.04+
sudo apt-get install libwebkit2gtk-4.1-dev

# Ubuntu 20.04
sudo apt-get install libwebkit2gtk-4.0-dev
```

### 问题 3: GTK3 头文件缺失

**错误信息**:
```
fatal error: gtk/gtk.h: No such file or directory
```

**解决方案**:
```bash
sudo apt-get install libgtk-3-dev
```

---

## 📦 依赖大小估算

完整安装后大约需要：
- **下载大小**: ~200 MB
- **磁盘占用**: ~500 MB

---

## 🚀 CI/CD 配置

在 GitHub Actions 中，使用以下配置：

```yaml
- name: Install system dependencies (Ubuntu)
  if: runner.os == 'Linux'
  run: |
    sudo apt-get update
    sudo apt-get install -y \
      protobuf-compiler \
      libwebkit2gtk-4.1-dev \
      build-essential \
      curl \
      wget \
      file \
      libssl-dev \
      libayatana-appindicator3-dev \
      librsvg2-dev \
      libgtk-3-dev \
      libsoup2.4-dev
```

---

## 🎯 最小依赖集合

如果要最小化安装，至少需要：

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  libssl-dev \
  libgtk-3-dev
```

但这会限制某些功能（如系统托盘、SVG 图标）。

---

## 📝 版本兼容性

| Ubuntu 版本 | webkit2gtk | GTK3 |
|-------------|-----------|------|
| 22.04+ | 4.1 | 3.24 |
| 20.04 | 4.0 | 3.22 |
| 18.04 | 2.24 | 3.22 |

**推荐**: Ubuntu 22.04 LTS 或更新版本

---

## 🔗 相关链接

- [Tauri Linux 依赖文档](https://tauri.app/v1/guides/getting-started/prerequisites/#linux)
- [WebKitGTK 官网](https://webkitgtk.org/)
- [GTK 官网](https://www.gtk.org/)

---

**最后更新**: 2025-01-14 (提交 39ed314)

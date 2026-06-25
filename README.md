# So Todo

So Todo 是一个仅面向 Windows 的待办事项应用，使用 Dioxus 0.7、Tailwind CSS 和 DaisyUI 构建。应用数据存储在本地 SQLite 单文件中，运行时资源会打包进可执行文件，发布包可以直接使用 `sotodo.exe`。

## 功能

- 未完成、日历、已完成三个视图
- 待办可选择是否设置日期、时间和提前提醒
- 支持到期时通知，以及到期前指定分钟数通知
- 未完成待办超过到期时间后标记为已超期
- 支持不指定日期/时间的待办，并在列表中单独分组
- 支持每周、每月等重复待办
- 描述内容支持多行展示，超过两行默认折叠，可点击展开
- Windows 系统托盘，支持显示主界面和退出
- 支持最小化到托盘、置顶、无原生标题栏拖动
- 支持开机自启动
- 默认跟随系统明暗主题，也可选择 DaisyUI 支持的主题
- 支持中文/英文界面，并可跟随系统语言
- 使用 SQLite 单文件存储，无需外部数据库
- 图标、CSS 等运行时资源内嵌到 exe

## 数据位置

数据库文件位于：

```text
%USERPROFILE%\.sotodo\sotodo.db
```

SQLite 通过 `rusqlite` 的 `bundled` 特性随应用编译，不需要额外安装 SQLite。

## 开发

安装 Rust 和 Dioxus CLI 后，可以启动桌面开发服务：

```powershell
dx serve --platform desktop
```

运行测试：

```powershell
cargo test
```

构建 Windows 桌面输出件：

```powershell
dx build --platform desktop --locked
```

输出目录：

```text
target\dx\sotodo\debug\windows\app\sotodo.exe
```

## 版本

本地开发版本显示为 `develop`。

通过 GitHub Actions 发布时，tag 名会写入应用内版本号，例如 `v0.0.2`。新增形如 `v*` 的 tag 并推送后，会触发 release workflow 构建并发布对应版本。

## 单文件输出

应用运行时需要的 CSS 和图标资源会内嵌到可执行文件中。`assets/` 目录仍然是编译期输入，但发布后的应用不依赖外部 `assets/` 文件夹。

## 项目结构

```text
assets/        编译期 CSS 和图标资源
src/main.rs    UI、状态管理、SQLite、托盘、自启、通知等主逻辑
Cargo.toml     Rust 依赖和 feature 配置
Dioxus.toml    Dioxus 桌面打包配置
.github/       GitHub Actions 构建和发布配置
```

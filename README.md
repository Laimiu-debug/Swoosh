# Swoosh

Swoosh 是面向个人用户的本地局域网传输软件。文件、文件夹、照片和文字，选一下，嗖过去。

应用使用 **Tauri 2 + Rust + React + TypeScript + Vite**，运行在独立桌面窗口中。发布版内嵌界面资源，无需启动浏览器或连接互联网。

## 当前可用功能

Windows 技术验证版已实现原生文件与文件夹选择、拖放、mDNS 设备发现、手动 IP 连接、文件与文字的 HTTPS 流式传输、两端短码核对与接收确认、进度、取消、SHA-256 校验、本地 SQLite 历史、收到文字后复制，以及浅深色主题。

文件保存到系统下载目录的 `Swoosh` 子目录。每次接收建立独立文件夹，保留目录结构与空目录，重复发送不会覆盖此前内容。私钥保存在应用本地数据目录，历史仅保留最近 100 项元数据，不保存文字正文。

当前范围为 Windows 首轮技术验证。Android、扫码、可信设备免确认、接收目录设置、断点续传和系统分享入口尚未交付。当前已通过同机双核心实例的真实 TLS 网络传输测试，两台独立电脑与真实路由器之间的验证需要后续补齐。

## 启动本地应用

开发环境需要 Node.js 22.12+、pnpm、Rust stable MSVC、Microsoft C++ Build Tools 和 WebView2。参见 [Tauri 官方环境要求](https://v2.tauri.app/start/prerequisites/)。

```sh
pnpm install --frozen-lockfile
pnpm desktop
```

Windows 也可以使用脚本自动补入当前进程的 Rust 路径：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/desktop.ps1
```

开发模式的 `127.0.0.1:1420` 只向桌面窗口提供热更新界面。发布版内嵌静态资源，与开发服务无关。`pnpm dev` 只用于界面调试，浏览器不具备原生传输能力。

## 打包与验证

```sh
pnpm check
cargo test -p swoosh-core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
pnpm package
```

Windows 安装程序输出在 `target/release/bundle/nsis/`；独立可执行文件为 `target/release/swoosh.exe`。

两台电脑启动后，在同一可互通网络内自动发现。发现不到时，从一端的「连接设备」复制 IP 与端口，在另一端输入。选择内容后点击目标设备，两端核对短码并确认，即可传输。网络首次提示时，由用户根据自己的网络环境决定是否允许应用访问专用网络。

协议使用专属 `_swoosh._tcp.local.` 服务类型和 HTTPS 接收端，默认端口 53318，被占用时自动选择空闲端口。当前协议不与 LocalSend 互通。每次会话均需确认，尚不保存信任或启用自动接收。

## 项目结构

```text
src/                       React 界面与状态展示
src-tauri/                 本地窗口、系统对话框与受限 IPC 命令
crates/swoosh-core/         身份、发现、TLS、传输、文件校验与 SQLite
design/                    设计令牌、品牌图标与独立设计预览
docs/                      需求、设计规范与开发进度
scripts/                   启动与资源生成脚本
```

## 开发与设计

- [开发需求文档](docs/development-requirements.md)：产品范围、技术栈、用户流程、协议约束、里程碑和验收标准。
- [开发进度与验证边界](docs/implementation-status.md)：当前已实现功能和后续工作。
- [设计系统](docs/design-system.md)：视觉方向、设计令牌、组件状态、布局和动效规范。
- [设计令牌源文件](design/tokens.json)：颜色、字号、间距、圆角、动效和浅深色主题。
- [CSS 变量](design/tokens.css)：由令牌源文件生成，可直接用于前端。
- [设计预览](design/preview.html)：可切换浅深色主题的界面与令牌预览，交互使用演示数据。
- [品牌图标](design/brand/README.md)：飞行「小信使」图标，包含 SVG、PNG、Windows ICO 和小尺寸预览。

设计预览是独立的视觉参考，应用代码位于 `src`、`src-tauri` 和 `crates`。

## 维护设计令牌

需要 Node.js 运行以下命令，无需安装 npm 依赖。

```sh
node scripts/generate-tokens.mjs
node scripts/generate-tokens.mjs --check
```

修改 `design/tokens.json` 后重新生成 CSS。直接用浏览器打开 `design/preview.html` 可查看设计效果。

也可以启动仅在本机监听的预览服务：

```sh
node scripts/preview-design.mjs
```

默认打开地址为 `http://127.0.0.1:4178`。

## 基础配置

- `.gitignore`：忽略本地环境变量、日志、临时文件和编辑器配置。
- `.editorconfig`：统一 UTF-8 编码、LF 换行和缩进规则。
- `.gitattributes`：统一文本文件换行，减少跨平台差异。

## 获取项目

```sh
git clone https://github.com/Laimiu-debug/Swoosh.git
cd Swoosh
```

依赖版本由 `pnpm-lock.yaml` 与 `Cargo.lock` 锁定。环境变量示例可以放在 `.env.example` 中，实际凭据应保存在被 Git 忽略的本地文件中。

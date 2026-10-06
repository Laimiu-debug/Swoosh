# 2026-10-06 商店准备验证

当前应用版本：0.1.1。MSIX 开发包版本：1.0.0.0，身份 `Swoosh.LocalPreview`。本次接收目录功能后已重建开发包，尚未提交过商店版本。本记录不是商店认证结果。

## 本次通过

- `pnpm check`：TypeScript、158 个设计令牌和品牌/Windows 图标一致性检查通过。
- `cargo test -p swoosh-core`：9 项通过，包括目录设置持久化/无效目录/默认恢复/失效回退，以及目录切换时当前接收保持原位置、新接收写入新位置、历史打开权限。
- `cargo fmt --all -- --check` 和 `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `pnpm package`：0.1.1 Windows x64 release 与 NSIS 安装程序构建成功，前端产物内嵌到 EXE。
- 原生桌面设置流程：底部「更改」打开弹窗；系统目录选择器从当前位置开始，选择后 SQLite 保存设置；「恢复默认」清除该设置，测试后保留默认位置。未通过此项推断完整双机或安装流程已经验收。
- Windows SDK 10.0.26100.0 MakeAppx：未使用跳过验证参数；清单、全信任应用入口、私有网络防火墙声明和 12 张包内图标成功打包。
- MakeAppx 解包：EXE SHA-256 与原始 release EXE 一致；中文清单描述正确保留 UTF-8。
- PE 依赖检查：EXE 导入为系统及系统 UCRT DLL，没有额外 VCRUNTIME/MSVCP DLL 依赖。此项不能替代干净机器和 WebView2 验收。
- 空的商店身份示例被脚本拒绝，没有生成可误提交的「真实商店身份」包。
- 官网隐私政策本地路由 HTTP 200，UTF-8 正文与页面元信息可读取。

## 本次产物

- NSIS 安装程序：`target/release/bundle/nsis/Swoosh_0.1.1_x64-setup.exe`，4,915,632 bytes，SHA-256 `4f49c133eab42104d8064a7e7eb4d3da1b375a569540072d5d3a14ef7223e149`。
- MSIX 文件：`target/msix/Swoosh_1.0.0.0_x64_local-preview.msix`
- 大小：6,837,505 bytes（约 6.52 MiB）
- 包 SHA-256：`e91c4a28c10501536b99ab58ee1106857a261f640197816feb97b8f51cf09c29`
- EXE SHA-256：`16d16fbfca95b79fbf1a3de1ef7cbe1afd36950d71310d1dc65f85f0637a2497`
- 机器可读报告：同目录同名 `.msix.json`
- 签名：未签名；开发身份包不可作为正式安装器或商店提交包。

## 尚未验证

Partner Center 实际产品身份、开发/商店签名后的 MSIX 安装升级卸载、最低系统版本与无 WebView2 的干净机器、最终候选包 Windows App Certification Kit、真实双机自动发现和大文件/断网流程均未完成。

此前商店准备时用户按 Esc 停止了桌面操作。本次通过 Computer Use 完成接收位置设置的原生界面验证；没有生成供公开上架使用的截图。商店截图仍待在测试设备与测试文件环境中实拍。

# 2026-10-06 商店准备验证

当前应用版本：0.1.0。MSIX 开发包版本：1.0.0.0，身份 `Swoosh.LocalPreview`。本记录不是商店认证结果。

## 本次通过

- `pnpm check`：TypeScript、158 个设计令牌和品牌/Windows 图标一致性检查通过。
- `pnpm tauri build --no-bundle`：Windows x64 release 构建成功，前端产物内嵌到 EXE。
- Windows SDK 10.0.26100.0 MakeAppx：未使用跳过验证参数；清单、全信任应用入口、私有网络防火墙声明和 12 张包内图标成功打包。
- MakeAppx 解包：EXE SHA-256 与原始 release EXE 一致；中文清单描述正确保留 UTF-8。
- PE 依赖检查：EXE 导入为系统及系统 UCRT DLL，没有额外 VCRUNTIME/MSVCP DLL 依赖。此项不能替代干净机器和 WebView2 验收。
- 空的商店身份示例被脚本拒绝，没有生成可误提交的「真实商店身份」包。
- 官网隐私政策本地路由 HTTP 200，UTF-8 正文与页面元信息可读取。

## 本次产物

- 文件：`target/msix/Swoosh_1.0.0.0_x64_local-preview.msix`
- 大小：6,793,409 bytes（约 6.48 MiB）
- 包 SHA-256：`2a2c8de504e42eb3cfc35e34565db6de1641abcefe79ecc38ee672513f253995`
- EXE SHA-256：`02dd804f7ea16f0fc3c87d459024ea426f41f8a6b11e8ab7e0785aac911c5dfd`
- 机器可读报告：同目录同名 `.msix.json`
- 签名：未签名；开发身份包不可作为正式安装器或商店提交包。

## 尚未验证

Partner Center 实际产品身份、开发/商店签名后的 MSIX 安装升级卸载、最低系统版本与无 WebView2 的干净机器、最终候选包 Windows App Certification Kit、真实双机自动发现和大文件/断网流程均未完成。

本次通过 Computer Use 启动本地应用后，用户按 Esc 停止了桌面操作。没有继续桌面操作、实拍截图或进行原生界面验收。商店截图仍待在测试设备与测试文件环境中实拍。

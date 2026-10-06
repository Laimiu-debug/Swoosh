# Swoosh Microsoft Store 上架准备

更新日期：2026-10-06。目标是 Windows 桌面 x64、USD 0.99 买断。尚未注册开发者账号、预留产品名或提交审核。

## 已准备

- `listing.zh-CN.md`：按实际能力编写的中文介绍、特性、系统要求和截图说明。
- `certification-notes.md`：审核双机流程及完整信任、局域网和防火墙权限用途。
- `assets/store-logo-300.png`：商店列表图标；包内图标见 `packaging/windows/Assets`。
- `../scripts/package-store.ps1`：使用 Windows SDK MakeAppx 制作 MSIX，执行 XML/资源验证及解包后的 EXE 哈希验证。
- `../website/dist/privacy.html`：官网隐私政策，涵盖应用、本机存储、局域网广播、第三方下载/商店及网站托管。
- `validation-2026-10-06.md`：本次成功构建、MakeAppx 验证、哈希、依赖检查与未验收事项。

## 开发包

在 Windows 上安装项目依赖、Rust MSVC 与 Windows SDK 后，从仓库根目录执行：

```powershell
pnpm package:store
```

默认构建 Tauri release，再生成 `target/msix/Swoosh_1.0.0.0_x64_local-preview.msix` 和同名 `.json` 验证报告。开发身份 `Swoosh.LocalPreview` 仅用于本地准备，不是 Partner Center 分配的产品身份，不可上传到商店。包未签名，不能直接双击当作正式安装器使用。

`-SkipBuild` 仅在已经构建且确认 release EXE 是当前源代码时使用：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-store.ps1 -SkipBuild
```

输出目录下保留独立 GUID 的暂存和解包目录用于检查。脚本不修改开发者模式，不导入证书，不安装/注册应用，不改本机防火墙设置。后续本机 MSIX 安装验收需要适当的开发注册或测试签名；不要把测试签名包作为商店正式包。

## 账号准备好之后

1. 用户本人从 https://storedeveloper.microsoft.com/ 创建账号并完成证件/自拍验证；按账号要求填写收款及税务资料。
2. 在 Partner Center 创建 **MSIX/PWA** 产品并检查 Swoosh 名称是否可预留。不要创建 EXE/MSI 产品来实现商店前置买断。
3. 在「产品管理 / 产品标识」复制 Package/Identity/Name、Package/Identity/Publisher 和 Package/Properties/PublisherDisplayName。
4. 把 `packaging/windows/store-identity.example.json` 复制到被 Git 忽略的 `.local/store-identity.json`，填入以上实际值。不要把空示例或猜测值提交给商店。
5. 生成使用真实商店身份的候选包：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-store.ps1 -StoreIdentityPath .local/store-identity.json
```

MSIX 版本独立于当前 Tauri 技术预览版本 0.1.1。微软要求商店版本的第一段非零、第四段为零，因此开发候选包从 1.0.0.0 开始；这不表示当前功能已经达到正式版验收。当前尚未提交过商店版本，本次开发包已包含接收位置设置；正式发布后更新需要递增适用包版本。

Microsoft Store 对通过审核的 MSIX 提供商店签名。当前脚本不会购买证书、保存证书私钥或生成商业签名。

## 收费提交前验收

| 项目 | 当前状态 | 验收要求 |
| --- | --- | --- |
| Release 构建与前端资源 | 已有成功构建；本次结果见验证报告 | 嵌入前端，脱离开发服务器启动 |
| MakeAppx 验证与包内容 | 本次通过，结果见验证报告 | 不跳过 schema 验证；解包 EXE 与 release 哈希一致 |
| 账号、产品身份、名称 | 待用户注册 | Partner Center 实际身份与 manifest 一致 |
| MSIX 安装/升级/卸载 | 待验收 | 当前用户安装、连续版本升级、卸载；下载目录收到的文件保留 |
| WebView2 与最低系统 | 待干净机验收 | Windows 10/11 的运行环境可用；MSIX 不依赖 NSIS 补装逻辑 |
| Windows App Certification Kit | 未运行 | 对最终可安装商店候选包运行并保存报告 |
| 双机与防火墙 | 待真实双机验收 | 专用网络自动发现、IP 连接，声明式防火墙规则生效 |
| 实际传输 | 同机两个独立身份测试通过 | 双机大文件、中文/空文件夹、短码确认、拒绝、取消与校验 |
| 中断及持久化 | 自动化测试有部分覆盖，实机待补 | 网络断开、应用退出/重启、不完整内容清理、历史恢复 |
| 商店截图 | 待实拍 | 至少一张真实 PNG，桌面最低 1366×768；计划四张 |

此前商店准备时的 Computer Use 由用户通过 Esc 停止；本次已验证接收位置设置的原生界面流程，尚未生成商店截图。截图不能用官网示意图替代。可在测试环境使用虚构设备名、自行创建的测试文件和文字实拍 `listing.zh-CN.md` 列出的四个场景。

## 最终提交设置

- 基础价 USD 0.99，检查各地区本地售价；一次性购买，无订阅。该设置在 Partner Center 完成，不是安装包里的价格开关。
- 首发支持简体中文、Windows 桌面 x64；最终系统范围以实机测试为准。
- 支持、官网和隐私政策链接见上架文案。广告、账号、手机端及未交付功能不得写成已支持。
- IARC 年龄分级问卷按真实内容回答，不能预先编造评级。
- 录入真实截图、图标、商店描述和审核说明，上传真实身份的最终候选包，完成验收后再提交认证。

## 参考

- [微软建议以 MSIX 分发并使用商店收款](https://learn.microsoft.com/en-us/windows/apps/publish/get-started)
- [个人开发者新注册流程](https://learn.microsoft.com/en-us/windows/apps/publish/whats-new-individual-developer)
- [MSIX 定价与 USD 0.99 档位](https://learn.microsoft.com/en-gb/windows/apps/publish/publish-your-app/msix/schedule-pricing-changes?pivots=store-installer-msix)
- [包要求与版本规则](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements)
- [MakeAppx](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool)
- [桌面包防火墙扩展](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/desktop-to-uwp-extensions)
- [商店截图要求](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images)
- [WebView2 分发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)

Tauri 官方商店指南主要介绍其原生 EXE/MSI 安装器路线。此项目采用微软 MakeAppx 额外包装成 MSIX，不能把 Tauri 原生 NSIS 商店配置直接当成 MSIX。

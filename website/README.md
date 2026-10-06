# Swoosh 官网

[公开官网](https://swoosh-laimiu.laimiu-new.chatgpt.site) · [Windows 预览版](https://github.com/Laimiu-debug/Swoosh/releases/tag/v0.1.1-preview.2)

官网沿用「小信使」品牌，提供功能介绍、文件与文字用途切换、三步使用说明、下载入口和常见问题。当前下载明确标记为 Windows 技术预览版，规划中的功能不作为已交付能力展示。

## 本机预览

在仓库根目录执行，无需安装网站框架或额外依赖：

```sh
node website/preview.mjs
```

也可以使用 `pnpm site`。打开 `http://127.0.0.1:4180/`。服务只监听本机；Ctrl+C 停止。

## 源码与发布

- `dist/index.html`、`style.css`、`site.js` 是直接发布的源码，无需构建。
- `dist/assets` 使用仓库已有品牌资源；没有外部字体、追踪脚本或表单服务。
- `dist/robots.txt` 与 `sitemap.xml` 使用实际官网地址。
- `.openai/hosting.json` 只保存 Sites 站点身份与静态目录配置，不包含发布凭据。
- Windows 安装包与 SHA-256 校验文件由 GitHub Releases 分发，二进制不纳入官网源码。

发布时使用独立的 Sites 源码检出目录，避免修改桌面项目的 Git 仓库。首次发布使用被忽略的 `.local/site-publish`；后续打开现有站点时应复用站点 ID，按 Sites 发布流程同步这里的源码、提交、推送、打包与部署，不要创建第二个站点或在 `website` 中建立嵌套 Git 仓库。

更新安装包时，先发布实际 GitHub Release，再同步首页版本、下载链接与版本说明。品牌改变时，从 `design/brand` 更新资源。站点只有静态介绍与下载功能；文件传输由独立的 Tauri 本地软件完成。

# Swoosh 品牌图标

品牌版本为 0.4。图形是一只向右飞行的小信使：扬起的翅膀和柔软的身体组成完整轮廓，保留一颗小眼睛和杏色短喙。用轻快、亲切的姿态表达“嗖地送到”，适合个人之间传文件与文字。

应用图标使用薄荷绿 `#087F6B`、纸白 `#FFFFFF`，内侧翅膀为浅薄荷 `#D8F1E7`，喙为浅杏色 `#FFD3AE`。不加渐变、速度线或文字；颜色均来自已有设计令牌。

## 资源

| 资源 | 用途 |
| --- | --- |
| [app-icon.svg](app-icon.svg) | 圆角应用图标，适合桌面与网页预览 |
| [app-icon-square.svg](app-icon-square.svg) | 无预设圆角的方形母版，供需要平台遮罩的后续适配使用 |
| [mark.svg](mark.svg) | 绿色单色标志，透明背景 |
| [mark-white.svg](mark-white.svg) | 白色单色标志，透明背景 |
| [favicon.svg](favicon.svg) | 小尺寸版，去掉翅膀内侧色块、放大眼睛 |
| [icon.ico](icon.ico) | Windows 图标，包含 16、32、48、64、128、256 px 六档 |
| [raster/swoosh-1024.png](raster/swoosh-1024.png) | 1024 px RGBA PNG，其他尺寸为 16、32、48、64、128、256、512 px |
| [raster/swoosh-square-1024.png](raster/swoosh-square-1024.png) | 方形 PNG 母版 |
| [preview.html](preview.html) | 小尺寸、单色和反白的浏览器预览 |

## 维护

[geometry.json](geometry.json) 是轮廓与构图源文件；颜色从 `design/tokens.json` 读取。修改源文件后运行：

```sh
node scripts/generate-brand.mjs
node scripts/generate-brand.mjs --check
```

生成脚本同步五份 SVG 和首页内联标志。PNG 与 ICO 可使用已安装的 Sharp 导出，同时自动同步 Windows 原生应用资源：

```sh
node scripts/generate-brand.mjs --sharp-dir /absolute/path/to/node_modules/sharp
```

Windows 应用、NSIS 安装器与卸载器已接入 `src-tauri/icons` 中的 PNG 和 ICO；`build.rs` 显式跟踪图标目录变化，让更新后的 ICO 重新编译进 Windows 程序资源。运行上述 Sharp 导出命令后，执行 `pnpm package` 重新构建应用与安装包。扩展移动平台时再使用 Tauri 的图标命令生成完整图标集。Tauri 接受方形 PNG 或 SVG 源图。[Tauri 图标文档](https://v2.tauri.app/develop/icons/)

## 使用规则

保持图形比例，不拉伸、不拆分翅膀、身体和喙。彩色图标的品牌色在浅深色界面保持一致；单色版本保留飞行轮廓，眼睛采用透明镂空，可随背景选择绿色或反白。图标外部至少留出图形宽度八分之一的空间。

小于 24 px 时优先使用 `favicon.svg` 或 16 px PNG；图标内不添加文字、描边或额外速度线。应用背景的遮罩与导出圆角由目标平台决定，避免重复裁切。

# 图片显示

Kenseader 在文章正文中嵌入式显示图片，图片出现在其原始位置。系统自动检测终端能力并选择最佳渲染方式。

## 支持的协议

| 协议 | 终端 | 质量 | 备注 |
|------|------|------|------|
| **Ueberzug++** | X11/Wayland 终端 | 最高（原生分辨率） | Linux 推荐 |
| **Kitty Graphics** | Kitty | 最高 | 原生协议 |
| **iTerm2 Inline** | iTerm2, WezTerm | 高 | macOS 原生 |
| **Sixel** | xterm, mlterm, foot, contour | 高 | 广泛支持 |
| **半块字符** | 所有支持真彩色的终端 | 中等（使用 `▀` 字符） | 通用回退 |

## Ueberzug++（Linux 推荐）

在 Linux 上获得最佳图片质量，请安装 [Ueberzug++](https://github.com/jstkdng/ueberzugpp)：

```bash
# Arch Linux
sudo pacman -S ueberzugpp

# Fedora
sudo dnf install ueberzugpp

# Ubuntu/Debian（从源码编译或使用 AppImage）
# 参见: https://github.com/jstkdng/ueberzugpp#installation
```

Ueberzug++ 将图片渲染为原生覆盖窗口，无论终端限制如何都能提供真正的高分辨率显示。Kenseader 在 X11/Wayland 环境中会自动检测并使用它。

## 工作原理

1. **自动检测** - 启动时自动检测终端能力和环境
2. **后端选择** - 自动选择最佳可用后端：
   - X11/Wayland + Ueberzug++：原生窗口覆盖（最高质量）
   - Kitty/iTerm2/WezTerm：原生终端协议
   - 回退：Unicode 半块字符（`▀`）
3. **可见优先加载** - 优先加载视口内的图片
4. **异步下载** - 图片在后台下载，不阻塞界面
5. **双层缓存** - 内存缓存快速访问 + 磁盘缓存持久化
6. **优雅降级** - 不支持高分辨率选项时自动回退到半块字符

## 全屏图片查看器

查看文章时，按 `Enter` 打开全屏图片查看器：

- **高分辨率显示** - 使用整个终端窗口以获得最大图片清晰度
- **图片间导航** - 使用 `n`/`p` 或方向键切换图片
- **外部打开** - 按 `o` 在系统默认图片查看器中打开
- **任意终端可用** - 即使使用半块字符，全屏模式也比内嵌显示提供更好的分辨率

## 配置选项

```toml
[ui]
image_preview = true  # 设为 false 完全禁用图片
```

## 终端兼容性

推荐使用支持原生图形协议的终端以获得最佳图片质量：

- **macOS**: iTerm2, Kitty, WezTerm
- **Linux**: Kitty, foot, WezTerm, GNOME Terminal（需启用 Sixel）
- **Windows**: Windows Terminal（通过 WSL 配合 Kitty/WezTerm）

对于不支持图形协议的终端，图片使用 Unicode 半块字符（`▀`）配合真彩色渲染。这在任何支持 24 位颜色的终端中都可以工作。

## 终端兼容性矩阵

| 终端 | macOS | Linux | Windows | 图片协议 |
|------|-------|-------|---------|----------|
| iTerm2   | ✅    | -     | -       | iTerm2 Inline  |
| Kitty    | ✅    | ✅    | -       | Kitty Graphics |
| WezTerm  | ✅    | ✅    | ✅      | iTerm2 Inline  |
| foot     | -     | ✅    | -       | Sixel          |
| GNOME Terminal | - | ✅  | -       | Sixel（需启用） |
| Windows Terminal | - | - | ✅     | 半块字符     |
| 其他   | ✅    | ✅    | ✅      | 半块字符     |

## 富文本渲染

文章内容经过解析后以格式化方式显示：

| 元素 | 显示样式 |
|------|----------|
| **标题** | 加粗，按级别着色（H1: 橙色, H2: 黄色, H3+: 青色） |
| **引用** | 斜体，带 `|` 前缀 |
| **代码** | 绿色文字，深色背景 |
| **列表** | 带 `•` 符号前缀 |
| **链接** | 内联显示 |
| **图片** | 在原始位置渲染 |

## 常见问题

### 图片不显示

1. 确保配置中 `image_preview = true`
2. 检查终端是否支持真彩色：`echo $COLORTERM` 应输出 `truecolor` 或 `24bit`
3. Linux 用户建议安装 Ueberzug++：`sudo pacman -S ueberzugpp`（Arch）或参见安装指南
4. macOS 用户建议使用 iTerm2、Kitty 或 WezTerm 以获得原生协议支持

### 图片加载慢

1. 图片是异步加载的，滚动时请稍等片刻
2. 检查网络连接
3. 部分网站会阻止图片外链

### 内存占用高

如果图片较多导致内存占用高：
- 图片缓存达到上限时会自动清理
- 重启应用可清空内存缓存
- 磁盘缓存会在会话间保留

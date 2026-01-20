# 云同步（iCloud/Dropbox 等）

将 RSS 数据同步到多设备（如 Mac + 未来的 iOS 应用）：

1. 编辑 `~/.config/kenseader/config.toml`
2. 设置 `data_dir` 为云存储路径：

   ```toml
   [general]
   # iCloud (macOS)
   data_dir = "~/Library/Mobile Documents/com~apple~CloudDocs/kenseader"

   # 或 Dropbox
   # data_dir = "~/Dropbox/kenseader"
   ```

3. 重启守护进程：`kenseader daemon stop && kenseader daemon start`

## 功能特性

- **波浪线展开**：路径支持 `~` 表示用户主目录（如 `~/Dropbox/kenseader`）
- **自动迁移**：修改 `data_dir` 时，现有数据会自动迁移到新位置
- **冲突检测**：如果新路径已存在数据库文件，守护进程会报错而非覆盖

## 同步内容

| 项目 | 是否同步 | 备注 |
|------|----------|------|
| 数据库 (`kenseader.db`) | 是 | 包含订阅源、文章、阅读状态、摘要等 |
| 图片缓存 (`image_cache/`) | 是 | 缓存的文章图片 |
| Socket 文件 (`kenseader.sock`) | 否 | 仅用于本地 IPC |
| PID 文件 (`daemon.pid`) | 否 | 本地进程跟踪 |

## 多设备同步的只读模式

使用云同步时，可以在**只读模式**下运行 TUI 来浏览文章，无需运行守护进程。适用场景：
- 在一台设备（如台式机）上运行守护进程，在另一台设备（如笔记本）上阅读
- 快速只读访问，无需启动守护进程
- 使用云同步时，另一台设备负责更新订阅源

```bash
# 以只读模式启动 TUI（无需守护进程）
kenseader run --read-mode
```

**只读模式功能：**
- 直接从同步的数据库浏览文章
- 可切换已读/未读状态（数据库锁定时自动重试写入）
- 可收藏/书签文章
- 状态栏和窗口标题显示 `[READ]` 指示器

**只读模式限制：**
- 无法刷新订阅源（由守护进程处理）
- 无法添加/删除订阅
- 数据库写入可能偶尔失败（如另一台设备正在写入时），会自动重试

**典型工作流：**
1. 在主设备上运行守护进程：`kenseader daemon start`
2. 在其他设备上使用只读模式：`kenseader run --read-mode`
3. 云同步保持所有设备的数据库同步

## 注意事项

- 配置文件（`~/.config/kenseader/config.toml`）不会被同步，保持本地独立
- 未来 iOS 开发：SQLite 数据库可以直接被 iOS 应用读取（使用 GRDB.swift 或 SQLite.swift 等库）

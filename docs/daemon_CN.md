# 后台守护进程

守护进程是**核心后端服务**，处理所有数据操作。TUI 是纯前端，通过 Unix socket IPC 与守护进程通信。

## 启动守护进程

```bash
# 启动后台守护进程
kenseader daemon start

# 检查守护进程状态
kenseader daemon status

# 停止守护进程
kenseader daemon stop
```

## 守护进程输出

启动守护进程后，你会看到：
```
Starting kenseader daemon...
Daemon started (PID: 12345). Press Ctrl+C or run 'kenseader daemon stop' to stop.
  Refresh interval: 300 seconds
  Cleanup interval: 3600 seconds
  Summarize interval: 60 seconds
  IPC socket: /Users/you/.local/share/kenseader/kenseader.sock
```

## 定时任务

| 任务 | 默认间隔 | 描述 |
|------|----------|------|
| **订阅源刷新** | 1 小时（调度器） | 智能刷新：仅获取超过单源间隔的订阅源 |
| **旧文章清理** | 1 小时 | 删除超过保留期限的文章 |
| **AI 摘要生成** | 1 分钟 | 为新文章生成摘要 |
| **文章过滤** | 2 分钟 | 评估文章相关性并自动过滤低相关性文章 |
| **风格分类** | 2 分钟 | 分类文章风格、语气和篇幅（与过滤同时运行） |

## 智能订阅源刷新

调度器使用智能的单源刷新间隔来减少不必要的网络请求：

- **调度器间隔** (`refresh_interval_secs`)：调度器检查需要刷新的订阅源的频率（默认：1 小时）
- **单源间隔** (`feed_refresh_interval_secs`)：每个订阅源两次刷新之间的最小时间（默认：12 小时）

只有当订阅源的 `last_fetched_at` 超过单源间隔时才会刷新。新订阅（从未获取过）会立即刷新。

## IPC API

守护进程通过 Unix socket 暴露以下操作：

| 方法 | 描述 |
|------|------|
| `ping` | 健康检查 |
| `status` | 获取守护进程状态和运行时间 |
| `feed.list` | 获取所有订阅源及未读数 |
| `feed.add` | 添加新订阅源 |
| `feed.delete` | 删除订阅源 |
| `feed.refresh` | 触发订阅源刷新 |
| `article.list` | 获取文章列表（支持过滤） |
| `article.get` | 通过 ID 获取单篇文章 |
| `article.mark_read` | 标记文章为已读 |
| `article.mark_unread` | 标记文章为未读 |
| `article.toggle_saved` | 切换收藏/书签状态 |
| `article.search` | 搜索文章 |

## 工作原理

1. **TUI 必需** - 启动 TUI 前必须先运行守护进程
2. **独立进程** - 守护进程与 TUI 分离运行，退出 TUI 后继续运行
3. **优雅退出** - 使用 `daemon stop` 或 Ctrl+C 正常停止
4. **PID 文件** - 守护进程 PID 保存在 `~/.local/share/kenseader/daemon.pid`
5. **IPC Socket** - Unix socket 位于 `~/.local/share/kenseader/kenseader.sock`
6. **可配置间隔** - 所有间隔都可在配置文件中自定义

## 配置选项

```toml
[sync]
refresh_interval_secs = 3600        # 调度器检查间隔（0 = 禁用）
feed_refresh_interval_secs = 43200  # 单源刷新间隔（12 小时）
cleanup_interval_secs = 3600        # 旧文章清理间隔
summarize_interval_secs = 60        # AI 摘要生成间隔
filter_interval_secs = 120          # 文章过滤间隔
```

设置 `refresh_interval_secs = 0` 可完全禁用后台调度器。
设置 `feed_refresh_interval_secs = 0` 则每次调度器运行时刷新所有订阅源。

## 测试 IPC 连接

可以用简单的 Python 脚本测试 IPC 连接：

```python
import socket
import json
import uuid

socket_path = "~/.local/share/kenseader/kenseader.sock"
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(socket_path)

# 发送 ping 请求
request = {"id": str(uuid.uuid4()), "method": "ping", "params": None}
sock.sendall((json.dumps(request) + "\n").encode())
print(sock.recv(4096).decode())  # {"id":"...","result":{"ok":true}}
```

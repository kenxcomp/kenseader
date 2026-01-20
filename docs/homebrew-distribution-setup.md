# Homebrew Distribution Setup Guide

本文档记录了为 Rust CLI 项目设置 Homebrew 分发的完整流程。

## 目标

让用户可以通过以下命令安装和使用：

```bash
brew tap kenxcomp/tap
brew install kenseader
brew services start kenseader  # 作为后台服务启动
kenseader run                   # 运行 TUI
```

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Release 流程                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   git push v0.1.0 tag                                          │
│         │                                                       │
│         ▼                                                       │
│   ┌─────────────────┐                                          │
│   │ GitHub Actions  │                                          │
│   │ release.yml     │                                          │
│   └────────┬────────┘                                          │
│            │                                                    │
│            ├──► Build (macOS arm64, x86_64, Linux x86_64)      │
│            │                                                    │
│            ├──► Create GitHub Release + Upload .tar.gz         │
│            │                                                    │
│            └──► Auto-update homebrew-tap/Formula/kenseader.rb  │
│                        │                                        │
│                        ▼                                        │
│            ┌─────────────────────┐                             │
│            │ kenxcomp/homebrew-tap │                            │
│            │ Formula/kenseader.rb │                            │
│            └─────────────────────┘                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 前置条件

- [x] 单一二进制文件项目
- [x] MIT/Apache 等开源许可证
- [x] GitHub 仓库

---

## 步骤 1：创建 GitHub Actions Release 工作流

**文件位置：** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            name: kenseader-macos-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            name: kenseader-macos-x86_64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: kenseader-linux-x86_64

    runs-on: ${{ matrix.os }}

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install dependencies (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y pkg-config libssl-dev

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package binary
        run: |
          cd target/${{ matrix.target }}/release
          tar -czvf ${{ matrix.name }}.tar.gz kenseader
          shasum -a 256 ${{ matrix.name }}.tar.gz > ${{ matrix.name }}.tar.gz.sha256

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}
          path: |
            target/${{ matrix.target }}/release/${{ matrix.name }}.tar.gz
            target/${{ matrix.target }}/release/${{ matrix.name }}.tar.gz.sha256

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            artifacts/**/*.tar.gz
            artifacts/**/*.sha256
          generate_release_notes: true

  update-homebrew:
    needs: release
    runs-on: ubuntu-latest

    steps:
      - name: Update Homebrew formula
        uses: mislav/bump-homebrew-formula-action@v3
        with:
          formula-name: kenseader
          homebrew-tap: kenxcomp/homebrew-tap
          download-url: https://github.com/${{ github.repository }}/releases/download/${{ github.ref_name }}/kenseader-macos-arm64.tar.gz
        env:
          COMMITTER_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
```

**关键点：**
- 触发条件：推送 `v*` 格式的 tag
- 构建三个平台：macOS ARM64、macOS x86_64、Linux x86_64
- 使用 `softprops/action-gh-release@v2` 创建 Release
- 使用 `mislav/bump-homebrew-formula-action@v3` 自动更新 Homebrew formula

---

## 步骤 2：添加 Daemon --foreground 选项

为支持 `brew services`，daemon 需要支持前台运行模式。

**修改 CLI 命令定义：**

```rust
// main.rs
#[derive(Subcommand)]
enum DaemonAction {
    /// Start the background daemon
    Start {
        /// Run in foreground (for launchd/systemd/brew services)
        #[arg(long)]
        foreground: bool,
    },
    // ...
}
```

**修改 daemon 启动逻辑：**

```rust
// daemon.rs
pub async fn start(db: Arc<Database>, config: Arc<AppConfig>, foreground: bool) -> Result<()> {
    if !foreground {
        // 仅在后台模式检查是否已运行
        if let Some(pid) = is_daemon_running() {
            println!("Daemon is already running (PID: {})", pid);
            return Ok(());
        }
    }

    // 仅在后台模式写入 PID 文件
    if !foreground {
        write_pid_file()?;
    }

    // ... daemon 主逻辑 ...

    // 清理时也检查模式
    if !foreground {
        remove_pid_file();
    }
}
```

**原因：**
- `brew services` 使用 launchd 管理进程生命周期
- launchd 需要进程在前台运行
- 前台模式下不需要 PID 文件管理

---

## 步骤 3：创建 Homebrew Tap 仓库

### 3.1 创建 GitHub 仓库

仓库命名必须为 `homebrew-tap` 格式：
- 仓库名：`kenxcomp/homebrew-tap`
- 这样用户可以用 `brew tap kenxcomp/tap` 添加

### 3.2 创建 Formula 文件

**文件位置：** `Formula/kenseader.rb`

```ruby
class Kenseader < Formula
  desc "High-performance terminal RSS reader with AI-powered summarization"
  homepage "https://github.com/kenxcomp/kenseader"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/kenxcomp/kenseader/releases/download/v#{version}/kenseader-macos-arm64.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    end
    on_intel do
      url "https://github.com/kenxcomp/kenseader/releases/download/v#{version}/kenseader-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/kenxcomp/kenseader/releases/download/v#{version}/kenseader-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "kenseader"
  end

  # brew services 支持
  service do
    run [opt_bin/"kenseader", "daemon", "start", "--foreground"]
    keep_alive true
    log_path var/"log/kenseader.log"
    error_log_path var/"log/kenseader.log"
  end

  def caveats
    <<~EOS
      To start the daemon manually:
        kenseader daemon start

      To use as a background service (recommended):
        brew services start kenseader

      To run the TUI:
        kenseader run

      Configuration file location:
        ~/.config/kenseader/config.toml
    EOS
  end

  test do
    assert_match "kenseader", shell_output("#{bin}/kenseader --help")
  end
end
```

**Formula 关键部分说明：**

| 部分 | 说明 |
|------|------|
| `on_macos/on_linux` | 平台条件判断 |
| `on_arm/on_intel` | CPU 架构判断 |
| `sha256` | 校验和，由 CI 自动更新 |
| `service do` | brew services 配置 |
| `keep_alive true` | 进程退出后自动重启 |
| `caveats` | 安装后显示的提示信息 |

### 3.3 创建 README

```markdown
# Homebrew Tap for kenxcomp

brew tap kenxcomp/tap

## Available Formulae

### kenseader

brew install kenseader
brew services start kenseader
```

---

## 步骤 4：配置 GitHub Secrets

### 4.1 创建 Personal Access Token (PAT)

**方式一：Fine-grained Token（推荐）**

1. GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens
2. Generate new token
3. 配置：
   - **Token name:** `homebrew-tap-update`
   - **Expiration:** 根据需要选择
   - **Repository access:** Only select repositories → 选择 `homebrew-tap`
   - **Permissions:**
     - Contents: Read and write
     - Metadata: Read-only（自动选中）

**方式二：Classic Token**

1. GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token (classic)
3. 配置：
   - **Note:** `homebrew-tap-update`
   - **Select scopes:** 勾选 `repo`

### 4.2 添加到主仓库 Secrets

1. 进入主项目仓库（kenseader）
2. Settings → Secrets and variables → Actions
3. New repository secret
   - **Name:** `HOMEBREW_TAP_TOKEN`
   - **Secret:** 粘贴 PAT

---

## 步骤 5：发布首个版本

```bash
# 1. 提交所有更改
git add -A
git commit -m "chore: add Homebrew distribution support"

# 2. 推送到远程
git push origin dev

# 3. 创建并推送 tag
git tag v0.1.0
git push origin v0.1.0
```

**触发流程：**
1. 推送 tag 触发 GitHub Actions
2. 构建三个平台的二进制文件
3. 创建 GitHub Release 并上传文件
4. 自动更新 homebrew-tap 中的 formula（更新 version 和 sha256）

---

## 步骤 6：验证

```bash
# 添加 tap
brew tap kenxcomp/tap

# 安装
brew install kenseader

# 验证安装
kenseader --help

# 启动服务
brew services start kenseader

# 检查服务状态
brew services list

# 运行 TUI
kenseader run
```

---

## 文件清单

| 文件 | 仓库 | 说明 |
|------|------|------|
| `.github/workflows/release.yml` | kenseader | CI/CD 工作流 |
| `crates/kenseader-cli/src/main.rs` | kenseader | CLI 命令定义 |
| `crates/kenseader-cli/src/commands/daemon.rs` | kenseader | Daemon 逻辑 |
| `Formula/kenseader.rb` | homebrew-tap | Homebrew formula |
| `README.md` | homebrew-tap | Tap 说明 |

---

## 后续版本发布

发布新版本只需：

```bash
# 更新 Cargo.toml 中的版本号（如果需要）

# 提交更改
git add -A
git commit -m "chore: bump version to 0.2.0"

# 创建新 tag
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

CI 会自动：
1. 构建新版本
2. 创建 GitHub Release
3. 更新 Homebrew formula 中的版本号和 SHA256

---

## 常见问题

### Q: brew services 启动失败？

检查日志：
```bash
cat /opt/homebrew/var/log/kenseader.log
# 或
brew services info kenseader
```

### Q: Formula SHA256 不匹配？

等待 CI 完成自动更新，或手动计算：
```bash
curl -sL <release-url> | shasum -a 256
```

### Q: 如何测试本地 formula？

```bash
brew install --build-from-source ./Formula/kenseader.rb
```

---

## 参考资料

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Tap Documentation](https://docs.brew.sh/Taps)
- [mislav/bump-homebrew-formula-action](https://github.com/mislav/bump-homebrew-formula-action)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)

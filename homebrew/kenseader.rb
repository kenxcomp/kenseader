# Homebrew formula for kenseader
# This file should be placed in the homebrew-tap repository at:
#   kenxcomp/homebrew-tap/Formula/kenseader.rb
#
# The SHA256 hashes will be automatically updated by the release workflow.

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

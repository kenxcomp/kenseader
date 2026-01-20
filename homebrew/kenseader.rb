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
      sha256 "1ca2c48257cb4f386432beb7bc02633702bb7bce4c06fa0a37b1197efcbf83aa"
    end
    on_intel do
      url "https://github.com/kenxcomp/kenseader/releases/download/v#{version}/kenseader-macos-x86_64.tar.gz"
      sha256 "4b50052f08fc0283420566113653df3fad38c42136c49cd65fa4e5b030a8788b"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/kenxcomp/kenseader/releases/download/v#{version}/kenseader-linux-x86_64.tar.gz"
      sha256 "aba564e85ea087b79a4fcd3d2b180ae5d5765cf7085c979e26dc1e04ee7e11ca"
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

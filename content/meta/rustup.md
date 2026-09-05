# 安装 rustup 与工具链管理

> 对应知识点：k0-2-1 · 预计 20 分钟 · 难度 ★☆☆

## 学习目标

- 完成 rustup 安装并验证
- 理解 rustc / cargo / rustup 三者的关系

## 一、三者关系

```text
rustup  ──── 工具链管理器（安装/切换/更新）
  ├── rustc    编译器
  ├── cargo    构建工具与包管理器
  ├── clippy   lint 工具（组件）
  ├── rustfmt  格式化工具（组件）
  └── rust-docs 本地文档（组件）
```

rustup 把以上工具安装在 `~/.cargo/bin`（该目录需在 PATH 中），所有版本切换、组件增删都通过 `rustup` 子命令完成。

## 二、安装

macOS / Linux：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 按提示选择默认安装（1），随后重启终端或 source "$HOME/.cargo/env"
```

Windows：下载 rustup-init.exe 或使用 winget。安装后验证：

```bash
rustc --version   # 编译器版本
cargo --version   # 构建工具版本
rustup show       # 默认工具链与安装位置
```

## 三、常用子命令

```bash
rustup update                 # 更新当前工具链
rustup update stable          # 只更新 stable
rustup toolchain install nightly   # 安装 nightly（按需）
rustup component add clippy rustfmt  # 添加组件
rustup doc                    # 打开本地离线文档
rustup default stable         # 切换默认工具链
```

## 四、stable 与 nightly

- **stable**（默认）：每 6 周一个版本，语法与 API 有向后兼容承诺，本教程全程使用
- **nightly**：实验特性入口，个别进阶工具（如某些 fuzz/miri 场景）需要它

## 验证清单

- [ ] `rustc --version` 与 `cargo --version` 均有输出
- [ ] `rustup show` 能看到 stable 工具链
- [ ] `rustup doc` 能打开浏览器文档

## 延伸阅读

- The Rust Book 1.1 Installation — https://doc.rust-lang.org/book/ch01-01-installation.html
- rustup Book — https://rust-lang.github.io/rustup/

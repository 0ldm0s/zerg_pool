# zerg_pool

[![License: WTFPL](https://img.shields.io/badge/License-WTFPL-brightgreen.svg)](http://www.wtfpl.net/about/)

高性能Rust线程池库，支持Unix socket和TCP双协议模式

## 特性

- 基于mio的高性能事件循环
- 支持动态worker管理
- 跨平台支持（Unix/Windows）
- 零成本抽象设计
- 线程安全的消息传递

## 快速开始

1. 添加依赖到`Cargo.toml`:
```toml
[dependencies]
zerg_pool = { git = "https://github.com/0ldm0s/zerg_pool.git" }
```

2. 基本用法示例:
```rust
use zerg_pool::ZergPool;

let pool = ZergPool::new(4); // 4个工作线程
pool.execute(|| {
    println!("Task running in worker thread");
});
```

## 构建

```bash
cargo build --release
```

## 运行测试

```bash
cargo test
```

## 许可证

本项目采用 [WTFPL](LICENSE) 许可证
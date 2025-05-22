#!/bin/bash
# 覆盖率测试脚本

# 安装必要工具
cargo install cargo-tarpaulin
rustup component add llvm-tools-preview

# 运行测试并生成lcov报告
cargo tarpaulin --output-dir ./target/coverage --engine llvm --ignore-tests

# 生成HTML报告
grcov ./target/coverage -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/coverage/html

# 打开报告
open ./target/coverage/html/index.html || \
    xdg-open ./target/coverage/html/index.html || \
    echo "请手动打开 ./target/coverage/html/index.html"
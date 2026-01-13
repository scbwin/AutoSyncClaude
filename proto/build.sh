#!/bin/bash

# Protocol Buffers 构建脚本
# 用于生成 Rust 代码

set -e

echo "Building Protocol Buffers definitions..."

# 创建输出目录
mkdir -p ../server/src/proto
mkdir -p ../client/src/proto

# 生成服务器端代码
echo "Generating server code..."
protoc \
    --rust_out=../server/src/proto \
    --tonic-out=../server/src/proto \
    --proto_path=. \
    sync.proto

# 生成客户端代码
echo "Generating client code..."
protoc \
    --rust_out=../client/src/proto \
    --tonic-out=../client/src/proto \
    --proto_path=. \
    sync.proto

echo "Protocol Buffers compilation completed!"
echo "Server code: ../server/src/proto/"
echo "Client code: ../client/src/proto/"

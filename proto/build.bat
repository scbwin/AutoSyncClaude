@echo off
REM Protocol Buffers 构建脚本 (Windows)
REM 用于生成 Rust 代码

echo Building Protocol Buffers definitions...

REM 创建输出目录
if not exist "..\server\src\proto" mkdir "..\server\src\proto"
if not exist "..\client\src\proto" mkdir "..\client\src\proto"

REM 生成服务器端代码
echo Generating server code...
protoc --rust_out=..\server\src\proto --tonic_out=..\server\src\proto --proto_path=. sync.proto

REM 生成客户端代码
echo Generating client code...
protoc --rust_out=..\client\src\proto --tonic_out=..\client\src\proto --proto_path=. sync.proto

echo Protocol Buffers compilation completed!
echo Server code: ..\server\src\proto\
echo Client code: ..\client\src\proto\

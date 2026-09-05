# TcpListener 与 TcpStream

> 对应知识点：k7-3-1 · 预计 35 分钟 · 难度 ★★☆

## 学习目标

- 用 std 实现一个能跑的 TCP echo 服务
- 理解字节流读取的 EOF 语义

## 一、服务端三步

```rust
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;   // 1. 绑定并监听
    println!("listening on 8080");
    for stream in listener.incoming() {                    // 2. 接受连接循环
        let mut stream = stream?;
        let mut buf = [0u8; 1024];
        loop {                                             // 3. 读写循环
            let n = stream.read(&mut buf)?;
            if n == 0 { break }                            // EOF = 对端关闭
            stream.write_all(&buf[..n])?;                  // 原样回显
        }
    }
    Ok(())
}
```

关键语义：

- `bind` 失败（端口占用）返回 Err → 用 `?` 处理
- `incoming()` 每次返回一个 `TcpStream`（对端 IP 可用 `peer_addr()` 查看）
- **`read` 返回 `Ok(0)` 表示对端关闭**（EOF），这是退出读取循环的唯一正确条件
- `read` 可能一次读不满（部分读），生产代码要循环读到期望长度

## 二、客户端

```rust
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    let mut s = TcpStream::connect("127.0.0.1:8080")?;
    s.write_all(b"hello")?;
    let mut buf = vec![0u8; 1024];
    let n = s.read(&mut buf)?;
    println!("echo: {}", String::from_utf8_lossy(&buf[..n]));
    Ok(())
}
```

## 三、验证与观察阻塞

1. 运行服务，`nc 127.0.0.1 8080`（或 telnet）输入任意文字验证回显
2. **开两个终端同时连接**：第二个会被挂起——因为服务端是单线程阻塞的，处理完第一个连接才 accept 下一个。这正是 k7-3-2（线程模型）与 k2-5-5（异步）要解决的问题
3. `ss -tlnp | grep 8080` 确认监听状态

## 验证清单

- [ ] echo 服务跑通并验证回显
- [ ] 复现「第二个连接被阻塞」现象并能解释原因
- [ ] 解释 read 返回 0 的含义

## 延伸阅读

- std::net 模块文档 — https://doc.rust-lang.org/std/net/
- The Rust Book 20.1（web server 案例）— https://doc.rust-lang.org/book/ch20-01-single-threaded-web-server.html

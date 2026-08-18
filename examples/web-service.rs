//! Web 场景：tiny_http 把请求交给 plugctx 服务。
//!
//! ```bash
//! cargo run -p plugctx-examples --example web-service --features web
//! curl -s http://127.0.0.1:3000/
//! ```
//!
//! 默认自检（`PLUGCTX_WEB_SELFTEST` 未设为 `0`）：临时端口上处理一次请求后 `dispose`。
//! 常驻：`PLUGCTX_WEB_SELFTEST=0 PLUGCTX_WEB_ADDR=127.0.0.1:3000 cargo run ...`

use std::io::{Read, Write};
use std::net::TcpStream;

use plugctx::{Context, Error, Plugin};
use tiny_http::{Response, Server};

struct GreetingPlugin;

impl Plugin for GreetingPlugin {
    fn build(&self, ctx: &mut Context) -> Result<(), Error> {
        ctx.provide("hello from plugctx".to_string());
        Ok(())
    }
}

fn reply(ctx: &Context, request: tiny_http::Request) {
    let body = ctx
        .get::<String>()
        .map(|s| s.clone())
        .unwrap_or_else(|| "missing".into());
    let _ = request.respond(Response::from_string(body));
}

fn main() {
    let ctx = Context::new();
    ctx.plugin(GreetingPlugin).expect("install");
    ctx.start().expect("start");

    let selftest = std::env::var("PLUGCTX_WEB_SELFTEST").unwrap_or_else(|_| "1".into()) != "0";
    let bind = if selftest {
        "127.0.0.1:0".to_string()
    } else {
        std::env::var("PLUGCTX_WEB_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into())
    };
    let server = Server::http(bind.as_str()).expect("bind");
    let ip = server.server_addr().to_ip().expect("IP listen addr");
    println!("listening {ip}");

    if selftest {
        let join = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(ip).expect("client connect");
            stream
                .write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
                .expect("write");
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
            buf
        });
        let request = server.incoming_requests().next().expect("one request");
        reply(&ctx, request);
        print!("{}", join.join().expect("client thread"));
    } else {
        for request in server.incoming_requests() {
            reply(&ctx, request);
        }
    }

    ctx.dispose();
}

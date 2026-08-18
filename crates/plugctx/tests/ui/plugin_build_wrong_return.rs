//! `Plugin::build` 不得返回错误的成功类型（须为 `Result<(), Error>`）。
use plugctx::{Context, Error, Plugin};

struct BadReturn;

impl Plugin for BadReturn {
    fn build(&self, _ctx: &mut Context) -> Result<i32, Error> {
        Ok(0)
    }
}

fn main() {}

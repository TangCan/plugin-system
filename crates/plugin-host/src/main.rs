//! Minimal demo CLI: load plugins from a directory and invoke an operation.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use plugin_host::{load_dir, LoadedPlugin};

fn usage() -> ! {
    eprintln!(
        "usage:\n  \
         plugin-host <plugin-dir> list\n  \
         plugin-host <plugin-dir> call <plugin-name> <op> [payload]\n\n\
         example:\n  \
         cargo run -p plugin-host -- target/debug list\n  \
         cargo run -p plugin-host -- target/debug call hello greet rust"
    );
    std::process::exit(2);
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(dir) = args.next() else {
        usage();
    };
    let Some(cmd) = args.next() else {
        usage();
    };

    let plugins = match load_dir(PathBuf::from(&dir)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("load_dir({dir}): {e}");
            return ExitCode::FAILURE;
        }
    };

    match cmd.as_str() {
        "list" => {
            if plugins.is_empty() {
                println!("(no plugins in {dir})");
            }
            for p in &plugins {
                println!("{:<16} {}", p.name(), p.path().display());
            }
            ExitCode::SUCCESS
        }
        "call" => {
            let Some(name) = args.next() else {
                usage();
            };
            let Some(op) = args.next() else {
                usage();
            };
            let payload = args.next().unwrap_or_default();
            match find(&plugins, &name) {
                Some(p) => match p.call(&op, payload.as_bytes()) {
                    Ok(out) => {
                        match String::from_utf8(out.clone()) {
                            Ok(s) => println!("{s}"),
                            Err(_) => {
                                println!("<{} bytes binary>", out.len());
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                },
                None => {
                    eprintln!("plugin `{name}` not found in {dir}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}

fn find<'a>(plugins: &'a [LoadedPlugin], name: &str) -> Option<&'a LoadedPlugin> {
    plugins.iter().find(|p| p.name() == name)
}

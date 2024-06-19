use std::{env, path::PathBuf};

use rusty_live_server::AsyncFileSystem;

const HELP: &str = "Usage: rusty-live-server PATH [OPTIONS]\n\nOptions:\n  -p, --port <PORT>  [default: 8080]\n  -h, --help         Print help\n  -V, --version      Print version";

#[tokio::main]
async fn main() {
    match parse_args() {
        Ok((path, port)) => {
            let afs = AsyncFileSystem::default();
            rusty_live_server::serve(path, port, true, None, afs)
                .await
                .unwrap()
        }
        Err(msg) => {
            println!("{msg}")
        }
    }
}

fn parse_args() -> Result<(PathBuf, u16), &'static str> {
    let args = env::args().skip(1);
    let mut path = vec![];
    let mut port: u16 = 8080;
    let mut next_port = false;
    for arg in args {
        if let Some(arg) = arg.strip_prefix('-') {
            if next_port {
                return Err("error: a value is required for '--port <PORT>' but none was supplied");
            }
            match arg {
                "-port" | "p" => {
                    next_port = true;
                }
                "-version" | "V" => return Err("rusty-live-server 0.4.0"),
                "-help" | "h" => return Err(HELP),
                _ => {
                    return Err("unkown argument");
                }
            };
        } else {
            match next_port {
                true => {
                    next_port = false;
                    port = arg.parse().map_err(|_| "Invalid port")?;
                }
                false => path.push(arg),
            }
        }
    }
    if next_port {
        return Err("error: a value is required for '--port <PORT>' but none was supplied");
    }
    if path.is_empty() {
        return Err(HELP);
    }
    if path.len() > 1 {
        return Err("error: more than 1 value provided");
    }

    Ok((PathBuf::from(path.pop().unwrap()), port))
}

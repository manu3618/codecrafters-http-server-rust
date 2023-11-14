use anyhow::anyhow;
use anyhow::Result;
use std::env::args;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::{thread, time};

fn handle_stream(mut stream: TcpStream, dir: &str) -> Result<()> {
    let mut buff = [0; 1024].to_vec();
    stream.read(&mut buff)?;
    let content = String::from_utf8(buff)?;
    let mut lines = content.lines();
    let b = lines.next().unwrap();
    if b.is_empty() {
        return Err(anyhow!("empty path"));
    }
    let path = b.split(' ').collect::<Vec<_>>()[1];

    // empy line
    lines.next();

    let mut _host = String::new();
    let mut user_agent = String::new();
    for _ in 0..2 {
        let b = lines.next().unwrap();
        let parts = b.split(' ').collect::<Vec<_>>();
        match parts[0] {
            "Host:" => _host = parts[1].into(),
            "User-Agent:" => user_agent = parts[1].into(),
            "Accept:" => (),
            "Accept-Encoding:" => (),
            "\r\n" => (),
            _ => {
                dbg!(&parts);
            }
        }
    }

    match handle_path(path, &user_agent, dir) {
        Ok(Served::Empty) => {
            let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
        }
        Ok(Served::String(response)) => {
            let echo = build_content(&response, "text/plain");
            let _ = stream.write(&echo.into_bytes());
        }
        Ok(Served::File(content)) => {
            let echo = build_content(&content, "application/octet-stream");
            let _ = stream.write(&echo.into_bytes());
        }
        _ => {
            let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
        }
    }
    Ok(())
}

enum Served {
    Empty,
    String(String),
    File(String),
}

fn handle_path(path: &str, user_agent: &str, dir: &str) -> Result<Served> {
    let parts: Vec<_> = path.split('/').collect();
    match parts.get(1) {
        Some(&"") => Ok(Served::Empty),
        Some(&"echo") => {
            if parts.len() < 2 {
                Ok(Served::Empty)
            } else {
                let r = parts[2..parts.len()].join("/");
                Ok(Served::String(r.clone()))
            }
        }
        Some(&"user-agent") => Ok(Served::String(String::from(user_agent))),
        Some(&"files") => {
            let r = parts[2..parts.len()].join("/");
            handle_file(&r, dir)
        }
        _ => Err(anyhow!("invalid path")),
    }
}

fn handle_file(path: &str, dir: &str) -> Result<Served> {
    let p = format!("{}/{}", dir, path);
    let p = Path::new(&p);
    let content = fs::read(p)?;
    let content = String::from_utf8(content)?;
    Ok(Served::File(content))
}

fn build_content(content: &str, content_type: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}\r\n",
        &content_type,
        &content.len(),
        &content
    )
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let mut previous_arg = String::new();
    let mut directory = String::new();
    for arg in args() {
        if previous_arg == "--directory" {
            directory = arg;
            break;
        } else {
            previous_arg = arg;
        }
    }

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let mut handlers = Vec::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir = directory.clone();
                handlers.push(thread::spawn(move || match handle_stream(stream, &dir) {
                    Ok(o) => {
                        dbg!(o);
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                }));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
        thread::sleep(time::Duration::from_millis(1));
    }
    for handle in handlers {
        let _ = handle.join();
    }
}

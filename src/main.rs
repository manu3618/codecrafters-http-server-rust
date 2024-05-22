use anyhow::anyhow;
use anyhow::Result;
use std::env::args;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::str::Lines;
use std::{thread, time};

enum Method {
    Get,
    Post,
}

#[derive(Debug, Default)]
enum Encoding {
    #[default]
    None,
    Invalid,
    Gzip,
}

#[derive(Debug, Default)]
struct Header {
    host: Option<String>,
    user_agent: Option<String>,
    accept_encoding: Encoding,
}

fn handle_stream(mut stream: TcpStream, dir: &str) -> Result<()> {
    let mut buff = [0; 1024].to_vec();
    let _ = stream.read(&mut buff)?;
    let content = String::from_utf8(buff)?;
    let mut lines = content.lines();
    let b = lines.next().unwrap();
    if b.is_empty() {
        return Err(anyhow!("empty path"));
    }
    let parts = b.split(' ').collect::<Vec<_>>();
    let method = match parts[0] {
        "GET" => Method::Get,
        "POST" => Method::Post,
        _ => unreachable!(),
    };
    let path = parts[1];

    let header = parse_header(&mut lines);
    dbg!(&header);

    match method {
        Method::Get => match handle_path(path, &header, dir) {
            Ok(Served::Empty) => {
                let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
            }
            Ok(Served::String(response)) => {
                let echo = build_content(&response, "text/plain", None);
                let _ = stream.write(&echo.into_bytes());
            }
            Ok(Served::File(content)) => {
                let echo = build_content(&content, "application/octet-stream", None);
                let _ = stream.write(&echo.into_bytes());
            }
            Ok(Served::Compressed(content)) => {
                let echo = build_content(&content, "text/plain", Some("gzip"));
                let _ = stream.write(&echo.into_bytes());
            }
            _ => {
                let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
            }
        },
        Method::Post => {
            let content = lines.collect::<Vec<_>>();
            dbg!(&content);
            let content = &content[2..content.len()].join("\r\n");
            let content = content.trim_matches(char::from(0));
            dbg!(&content);
            match write_file(content, path, dir) {
                Ok(()) => {
                    let _ = stream.write(b"HTTP/1.1 201 Createdi\r\n\r\n")?;
                    return Ok(());
                }
                Err(e) => {
                    let _ = stream.write(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")?;
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

enum Served {
    Empty,
    String(String),
    File(String),
    Compressed(String),
}

fn parse_header(lines: &mut Lines) -> Header {
    let mut header = Header::default();
    let max_header_len = 5;
    for _ in 0..max_header_len {
        let b = lines.next().unwrap();
        let parts = b.split(' ').collect::<Vec<_>>();
        match parts[0].to_lowercase().as_str() {
            "host:" => header.host = Some(parts[1].into()),
            "user-agent:" => header.user_agent = Some(parts[1].into()),
            "accept:" => (),
            "accept-encoding:" => {
                header.accept_encoding = match parts[1] {
                    "gzip" => Encoding::Gzip,
                    _ => {
                        dbg!("unknown encoding {}", &parts[0]);
                        Encoding::Invalid
                    }
                }
            }
            "\r\n" | "" => break, // end of header
            _ => {
                dbg!(&parts);
            }
        }
    }
    header
}

fn handle_path(path: &str, header: &Header, dir: &str) -> Result<Served> {
    let parts: Vec<_> = path.split('/').collect();
    match parts.get(1) {
        Some(&"") => Ok(Served::Empty),
        Some(&"echo") => match header.accept_encoding {
            Encoding::Gzip => Ok(Served::Compressed(path.into())),
            _ => {
                if parts.len() < 2 {
                    Ok(Served::Empty)
                } else {
                    let r = parts[2..parts.len()].join("/");
                    Ok(Served::String(r.clone()))
                }
            }
        },
        Some(&"user-agent") => Ok(Served::String(
            header.user_agent.clone().unwrap_or("".into()),
        )),
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

fn build_content(content: &str, content_type: &str, encoding: Option<&str>) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("HTTP/1.1 200 OK".into());
    if let Some(e) = encoding {
        lines.push(format!("Content-Encoding: {}", e));
    }
    lines.push(format!("Content-Type: {}", &content_type));
    lines.push(format!("Content-Length: {}", &content.len()));
    lines.push("".into());
    lines.push(content.into());
    lines.push("".into());

    lines.join("\r\n")
}

fn write_file(content: &str, path: &str, dir: &str) -> Result<()> {
    dbg!(&path, &dir);
    let path = &path[7..path.len()];
    let p = format!("{}{}", dir, path);
    let p = Path::new(&p);
    dbg!(&p);
    let parent = p.parent();
    fs::create_dir_all(parent.unwrap())?;
    eprintln!("writing to {:?}", &p);
    eprintln!("file content:\n{:?}", &content);
    fs::write(p, content.as_bytes())?;
    Ok(())
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

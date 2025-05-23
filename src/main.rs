use anyhow::anyhow;
use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
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

#[derive(Debug, Default, PartialEq)]
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
    accept_encoding: Vec<Encoding>,
}

fn handle_stream(mut stream: TcpStream, dir: &str) -> Result<()> {
    let mut buff = [0; 2048].to_vec();
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
                let echo = build_content(&response, "text/plain", None, None);
                let _ = stream.write(&echo.into_bytes());
            }
            Ok(Served::File(content)) => {
                let echo = build_content(&content, "application/octet-stream", None, None);
                let _ = stream.write(&echo.into_bytes());
            }
            Ok(Served::Compressed(content)) => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&content.into_bytes())?;
                let compressed = encoder.finish()?;
                // let h = hex::encode(&compressed);

                let mut echo = build_content_header("text/plain", Some("gzip"), compressed.len());
                echo.push_str("\r\n");
                let mut echo = echo.into_bytes();
                echo.extend(compressed);

                let _ = stream.write(&echo);
            }
            _ => {
                let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
            }
        },
        Method::Post => {
            let content = lines.collect::<Vec<_>>();
            dbg!(&content);
            let content = &content.join("\r\n");
            let content = content.trim_matches(char::from(0));
            dbg!(&content);
            match write_file(content, path, dir) {
                Ok(()) => {
                    let _ = stream.write(b"HTTP/1.1 201 Created\r\n\r\n")?;
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
                for e in parts[1..].iter() {
                    header.accept_encoding.push(match e.trim_matches(',') {
                        "gzip" => Encoding::Gzip,
                        _ => {
                            dbg!(e);
                            dbg!(&parts);
                            Encoding::Invalid
                        }
                    })
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
        Some(&"echo") => {
            if header.accept_encoding.contains(&Encoding::Gzip) && parts.len() >= 2 {
                let r = parts[2..].join("/");
                Ok(Served::Compressed(r))
            } else if parts.len() < 2 {
                Ok(Served::Empty)
            } else {
                let r = parts[2..parts.len()].join("/");
                Ok(Served::String(r.clone()))
            }
        }

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

fn build_content(
    content: &str,
    content_type: &str,
    encoding: Option<&str>,
    content_length: Option<usize>,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let len = content_length.unwrap_or(content.len());
    lines.push(build_content_header(content_type, encoding, len));
    lines.push(content.into());
    lines.push("".into());
    lines.join("\r\n")
}

fn build_content_header(
    content_type: &str,
    encoding: Option<&str>,
    content_length: usize,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("HTTP/1.1 200 OK".into());
    if let Some(e) = encoding {
        lines.push(format!("Content-Encoding: {}", e));
    }
    lines.push(format!("Content-Type: {}", &content_type));
    lines.push(format!("Content-Length: {}", content_length));
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

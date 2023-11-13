use anyhow::anyhow;
use anyhow::Result;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

fn handle_stream(mut stream: TcpStream) -> Result<()> {
    let mut buff = String::with_capacity(256);
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut buff)?;
    let b = buff.clone();
    if b.len() == 0 {
        return Err(anyhow!("empy path"))
    }
    let path = b.split(' ').collect::<Vec<_>>()[1];

    // empy line
    buff.clear();
    reader.read_line(&mut buff)?;

    let mut _host = String::new();
    let mut user_agent = String::new();
    for _ in 0..2 {
        buff.clear();
        reader.read_line(&mut buff)?;
        let b = buff.clone();
        let parts = b.split(' ').collect::<Vec<_>>();
        match parts[0] {
            "Host:" => _host = parts[1].into(),
            "User-Agent:" => user_agent = parts[1][..parts[1].len() - 2].into(),
            "Accept:" => (),
            "Accept-Encoding:" => (),
            "\r\n" => (),
            _ => {
                dbg!(&parts);
            }
        }
    }

    match handle_path(path, &user_agent) {
        Ok(None) => {
            let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
        }
        Ok(Some(response)) => {
            let echo = build_content(&response);
            let _ = stream.write(&echo.into_bytes());
        }
        _ => {
            let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
        }
    }
    Ok(())
}

fn handle_path(path: &str, user_agent: &str) -> Result<Option<String>> {
    let parts: Vec<_> = path.split('/').collect();
    // dbg!(&parts);
    match parts.get(1) {
        Some(&"") => Ok(None),
        Some(&"echo") => {
            if parts.len() < 2 {
                Ok(None)
            } else {
                let r = parts[2..parts.len()].join("/");
                Ok(Some(r.clone()))
            }
        }
        Some(&"user-agent") => Ok(Some(String::from(user_agent))),
        _ => Err(anyhow!("invalid path")),
    }
}

fn build_content(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
        &content.len(),
        &content
    )
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let mut handlers = Vec::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handlers.push(thread::spawn(|| handle_stream(stream)));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    for handle in handlers {
        let _ = handle.join();
    }
}

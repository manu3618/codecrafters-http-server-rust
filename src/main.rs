use anyhow::Result;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

fn handle_stream(stream: &mut TcpStream) -> Result<()> {
    let mut buff = String::new();
    dbg!(&stream);
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut buff)?;
    dbg!(&buff);
    let parts: Vec<_> = buff.split(' ').collect();
    if parts.get(1) == Some(&"/") {
        let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
    } else {
        let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
    }
    Ok(())
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let _ = handle_stream(&mut stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use anyhow::Result;

fn handle_stream(stream: &mut TcpStream) -> Result<()> {
    let mut buff = [0;20];
    dbg!(&stream);
    stream.read_exact(&mut buff)?;
    let buff = String::from_utf8(buff.to_vec())?;
    if buff.starts_with("GET / HTTP") {
        dbg!("/");
        let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
    } else {
        dbg!("pas /");
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

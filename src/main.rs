use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

fn handle_stream(stream: &mut TcpStream) -> std::io::Result<()> {
    let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
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

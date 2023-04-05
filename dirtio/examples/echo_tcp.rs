use dirtio::net::tcp::TcpListener;
use futures::{AsyncReadExt, AsyncWriteExt};

#[dirtio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";

    let mut listener = TcpListener::bind(addr.parse().unwrap())?;
    println!("listening on: {}", addr);

    loop {
        let (mut stream, _) = listener.accept().await?;

        dirtio::spawn(async move {
            let mut buf = vec![0; 4096];

            loop {
                match stream.read(&mut buf).await {
                    Err(e) => {
                        println!("failed to read data: {}", e);
                        break;
                    }
                    Ok(n) if n == 0 => break,
                    Ok(n) => {
                        if let Err(e) = stream.write_all(&buf[0..n]).await {
                            println!("failed to write data: {}", e);
                            break;
                        }
                    }
                }
            }
        });
    }
}

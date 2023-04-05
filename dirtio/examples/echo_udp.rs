use dirtio::net::udp::UdpSocket;

#[dirtio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let mut buf = [0; 65535];
    let socket = UdpSocket::bind(addr.parse().unwrap())?;

    println!("listening on: {}", addr);

    loop {
        let (n, addr) = socket.recv_from(&mut buf).await?;
        match socket.send_to(&buf[..n], addr).await {
            Err(e) => {
                println!("failed to echo to {}: {}", addr, e);
            }
            Ok(_) => {
                println!("echoed {} bytes to {}", n, addr);
            }
        }
    }
}

use std::net::SocketAddr;

use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};

async fn handle_tcp_stream(mut tcp_stream: TcpStream) -> anyhow::Result<()> {

    let mut buf = [0; 16];

    loop {

        let bytes_read = tcp_stream.read(&mut buf).await?;

        if bytes_read == 0 {
            break;
        }

        tcp_stream.write_all(&buf[0..bytes_read]).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Test");

    let addr = "0.0.0.0:5001".parse::<SocketAddr>()?;
    let listner = tokio::net::TcpListener::bind(addr).await?;

    loop {
        let (tcp_stream, _) = listner.accept().await?;
        tokio::spawn( async move {
            handle_tcp_stream(tcp_stream).await
        });
    }
}

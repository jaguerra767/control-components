use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;
use crate::controllers::clear_core::Message;

pub async fn client<T: ToSocketAddrs>(addr: T, mut msg: mpsc::Receiver<Message>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(addr).await?;
    while let Some(message) = msg.recv().await {
        stream.write_all(&message.buffer).await?;
        stream.readable().await?;
        let mut buffer = [0; 100];
        match stream.read(&mut buffer).await {
            Ok(0) => {
                eprintln!("Connection closed by server");
            }
            Ok(_) => {
                if message.response.send(buffer.to_vec()).is_err() {
                    eprintln!("Failed to send via channel");
                }
            }
            Err(e) => {
                eprintln!("Failed to read from stream: {}", e);
                break;
            }
        }
    }
    Ok(())
}
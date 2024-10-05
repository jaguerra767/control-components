use crate::controllers::clear_core::Message;
use log::{error, info};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;

pub async fn client<T: ToSocketAddrs>(
    addr: T,
    mut msg: mpsc::Receiver<Message>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(addr).await?;
    let peer_addr = stream.peer_addr().expect(" Peer not connected");
    info!("Client connected with peer address: {peer_addr}");
    while let Some(message) = msg.recv().await {
        stream.write_all(&message.buffer).await?;
        stream.readable().await?;
        let mut buffer = [0; 100];
        match stream.read(&mut buffer).await {
            Ok(0) => {
                error!("Connection closed by server");
            }
            Ok(_) => {
                if message.response.send(buffer.to_vec()).is_err() {
                    error!("Failed to send via channel");
                }
            }
            Err(e) => {
                error!("Failed to read from stream: {}", e);
                break;
            }
        }
    }
    Ok(())
}

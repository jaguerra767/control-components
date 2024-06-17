use std::error::Error;
use tokio::sync::{mpsc, oneshot};
use crate::controllers::clear_core::Message;

pub trait SendRecv {
    fn get_sender(&self) -> &mpsc::Sender<Message>;
    fn write(
        &self, buffer: &[u8]) -> impl std::future::Future<Output = Result<Vec<u8>, Box<dyn Error>>> + Send where Self:Sync {async {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = Message { buffer: buffer.to_vec(), response: resp_tx };
        self.get_sender().send(msg).await?;
        let res = resp_rx.await?;
        Ok(res) } 
    }
}


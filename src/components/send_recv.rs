use crate::controllers::clear_core::Message;
use std::future::Future;
use tokio::sync::{mpsc, oneshot};

pub trait SendRecv {
    fn get_sender(&self) -> &mpsc::Sender<Message>;
    fn write(&self, buffer: &[u8]) -> impl Future<Output = Vec<u8>>
    where
        Self: Sync,
    {
        async {
            let (resp_tx, resp_rx) = oneshot::channel();
            let msg = Message {
                buffer: buffer.to_vec(),
                response: resp_tx,
            };
            self.get_sender().send(msg).await.expect("Failed to send msg to client");
            resp_rx.await.expect("No MSG from client")
        }
    }
}

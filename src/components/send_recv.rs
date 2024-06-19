use crate::controllers::clear_core::Message;
use std::error::Error;
use std::future::Future;
use tokio::sync::{mpsc, oneshot};

pub trait SendRecv {
    fn get_sender(&self) -> &mpsc::Sender<Message>;
    //fn get_receiver(&self) -> mpsc::Receiver<Message>;
    fn write(&self, buffer: &[u8]) -> impl Future<Output = Result<Vec<u8>, Box<dyn Error>>> + Send
    where
        Self: Sync,
    {
        async {
            let (resp_tx, resp_rx) = oneshot::channel();
            let msg = Message {
                buffer: buffer.to_vec(),
                response: resp_tx,
            };
            self.get_sender().send(msg).await?;
            let res = resp_rx.await?;
            //println!("{:?}", res);
            Ok(res)
        }
    }

    // fn get_ip_address<T: ToSocketAddrs>(&self) -> T;
    // fn get_client<T: ToSocketAddrs + Sync + Send>(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send where Self:Sync {
    //     async {
    //         let mut stream = TcpStream::connect(self.get_ip_address::<T>()).await?;
    //         let mut rx = self.get_receiver();
    //         while let Some(message) = rx.recv().await {
    //             stream.write_all()
    //         }
    //         Ok(())
    //     }
    // }
}

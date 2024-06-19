use std::error::Error;
use tokio::sync::{mpsc, oneshot};

pub const STX: u8 = 2;
pub const CR: u8 = 13;
pub const RESULT_IDX: u8 = 3;

pub struct Message {
    pub buffer: Vec<u8>,
    pub response: oneshot::Sender<Vec<u8>>,
}

pub struct Controller {
    sender: mpsc::Sender<Message>,
}

impl Controller {
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        Controller { sender }
    }
    pub async fn write(&self, buffer: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = Message {
            buffer: buffer.to_vec(),
            response: resp_tx,
        };
        self.sender.send(msg).await?;
        let res = resp_rx.await?;
        Ok(res)
    }
}

#[tokio::test]
async fn test_controller() {
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    let mock_client = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if msg.response.send(msg.buffer).is_err() {
                eprintln!("Unable to send Response");
            }
        }
    });

    let controller_task_1 = tokio::spawn(async move {
        let controller = Controller::new(tx);
        let reply = controller.write("Test_1".as_bytes()).await.expect("Failed");
        println!("{:?}", reply);
        assert_eq!(reply.as_slice(), "Test_1".as_bytes());
    });

    let controller_task_2 = tokio::spawn(async move {
        let controller = Controller::new(tx2);
        let reply = controller.write("Test_2".as_bytes()).await.expect("Failed");
        println!("{:?}", reply);
        assert_eq!(reply.as_slice(), "Test_2".as_bytes());
    });

    let controller_task_3 = tokio::spawn(async move {
        let controller = Controller::new(tx3);
        let reply = controller.write("Test_3".as_bytes()).await.expect("Failed");
        println!("{:?}", reply);
        assert_eq!(reply.as_slice(), "Test_3".as_bytes());
    });

    mock_client.await.unwrap();
    controller_task_1.await.unwrap();
    controller_task_2.await.unwrap();
    controller_task_3.await.unwrap();
}

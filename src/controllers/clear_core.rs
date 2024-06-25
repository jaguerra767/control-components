use tokio::net::ToSocketAddrs;
use tokio::sync::{oneshot};
use tokio::sync::mpsc::{channel, Sender};
use crate::components::clear_core_io::{Input, Output};
use crate::components::clear_core_motor::ClearCoreMotor;
use crate::interface::tcp::client;


pub const STX: u8 = 2;
pub const CR: u8 = 13;
pub const RESULT_IDX: u8 = 3;


const NO_INPUTS: usize = 7;
const NO_OUTPUTS: usize = 6;

pub struct Message {
    pub buffer: Vec<u8>,
    pub response: oneshot::Sender<Vec<u8>>,
}

pub type Motors = Vec<ClearCoreMotor>;
pub type Inputs = Vec<Input>;

pub type Outputs = Vec<Output>;

pub struct MotorBuilder {
    pub id: u8,
    pub scale: usize
}

//The way controller is meant to be used now is to feed it the "recipe" for how to make a motor 
//(id and scale) and a single tx that the constructor then copies so that we don't have to copy it 
//ourselves and worry about it being dropped correctly. 

pub struct Controller {
    motors: Motors,
    inputs: Inputs,
    outputs: Outputs
}

impl Controller {
    //New now moves in the tx made by mpsc::channel() so that we only need to move it once, and we
    //can forget about it, the other parameter takes an array of the MotorBuilder struct. This allows
    //us to define the scale and id of a motor as a const in the config file, so that all we have to
    //change now is in that file. Something we can do in the future is make a HashMap of controllers
    //with a name and associate a sender to that but that seems like overkill to me now.
    pub fn new(tx: Sender<Message>, motors: &[MotorBuilder]) -> Self {
        let motors = motors.into_iter()
            .map(|motor|{ ClearCoreMotor::new(motor.id, motor.scale, tx.clone()) })
            .collect();
        let inputs = (0..NO_INPUTS).into_iter()
           .map(|index|{ Input::new(index as u8, tx.clone())})
            .collect();
        let outputs = (0..NO_OUTPUTS).into_iter()
            .map(|index|{ Output::new(index as u8, tx.clone())})
            .collect();

        Controller { motors, inputs, outputs }
    }
    pub fn get_motor(&self, id: usize) -> Option<&ClearCoreMotor> {
        self.motors.get(id)
    }
    
    pub fn get_inputs(&self, id: usize) -> Option<&Input> {
        self.inputs.get(id)
    }
    
    pub fn get_output(&self, id: usize) -> Option<&Output> {
        self.outputs.get(id)
    }
        
}

//TODO: RENAME!!!!!!!

pub async fn actor<T: ToSocketAddrs + Sync>(addr: &'static T) {
    let (tx, rx) = channel(100);
    let client_handle = tokio::spawn(async  {
        //we can use expect because it's totally cool to panic here
       client(addr, rx).await.expect("TODO: panic message"); 
    });
    //TODO: WIP DON"T USE
}


// #[tokio::test]
// async fn test_controller() {
//     let (tx, mut rx) = mpsc::channel::<Message>(100);
//     let tx2 = tx.clone();
//     let tx3 = tx.clone();
// 
//     let mock_client = tokio::spawn(async move {
//         while let Some(msg) = rx.recv().await {
//             if msg.response.send(msg.buffer).is_err() {
//                 eprintln!("Unable to send Response");
//             }
//         }
//     });
// 
//     let controller_task_1 = tokio::spawn(async move {
//         let controller = Controller::new(tx);
//         let reply = controller.write("Test_1".as_bytes()).await.expect("Failed");
//         println!("{:?}", reply);
//         assert_eq!(reply.as_slice(), "Test_1".as_bytes());
//     });
// 
//     let controller_task_2 = tokio::spawn(async move {
//         let controller = Controller::new(tx2);
//         let reply = controller.write("Test_2".as_bytes()).await.expect("Failed");
//         println!("{:?}", reply);
//         assert_eq!(reply.as_slice(), "Test_2".as_bytes());
//     });
// 
//     let controller_task_3 = tokio::spawn(async move {
//         let controller = Controller::new(tx3);
//         let reply = controller.write("Test_3".as_bytes()).await.expect("Failed");
//         println!("{:?}", reply);
//         assert_eq!(reply.as_slice(), "Test_3".as_bytes());
//     });
// 
//     mock_client.await.unwrap();
//     controller_task_1.await.unwrap();
//     controller_task_2.await.unwrap();
//     controller_task_3.await.unwrap();
// }

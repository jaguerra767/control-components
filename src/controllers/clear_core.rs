use crate::components::clear_core_io::{AnalogInput, DigitalInput, DigitalOutput, HBridge};
use crate::components::clear_core_motor::{ClearCoreMotor, Status};
use crate::interface::tcp::client;
use std::error;
use std::fmt;
use std::fmt::Formatter;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc::channel;
use tokio::sync::oneshot;
use tokio::task::JoinSet;

pub const STX: u8 = 2;
pub const CR: u8 = 13;
pub const RESULT_IDX: u8 = 3;

const NO_DIGITAL_INPUTS: usize = 3;
const NO_ANALOG_INPUTS: usize = 4;
const NO_OUTPUTS: usize = 6;
const NO_HBRIDGE: usize = 2;

pub struct Message {
    pub buffer: Vec<u8>,
    pub response: oneshot::Sender<Vec<u8>>,
}

//TODO: Change to arrays using array::from_fn
pub type Motors = Vec<ClearCoreMotor>;
pub type Inputs = Vec<DigitalInput>;

pub type AnalogInputs = Vec<AnalogInput>;
pub type Outputs = Vec<DigitalOutput>;
pub type HBridges = [HBridge; NO_HBRIDGE];

const REPLY_IDX: usize = 3;
const FAILED_REPLY: u8 = b'?';

pub struct MotorBuilder {
    pub id: u8,
    pub scale: usize,
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl<T: error::Error + Send + Sync + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}
pub async fn check_reply(reply: &[u8]) -> Result<(), Error> {
    if reply[REPLY_IDX] == FAILED_REPLY {
        Err(Error {
            message: std::str::from_utf8(reply)?.to_string(),
        })
    } else {
        Ok(())
    }
}

pub struct ControllerHandle {
    motors: Motors,
    digital_inputs: Inputs,
    analog_inputs: AnalogInputs,
    outputs: Outputs,
    h_bridges: HBridges,
}

impl ControllerHandle {
    pub fn new<T>(addr: T, motors: [MotorBuilder; 4]) -> Self
    where
        T: ToSocketAddrs + Send + 'static,
    {
        let (tx, rx) = channel::<Message>(10);
        tokio::spawn(async move {
            client(addr, rx).await.unwrap();
        });
        let motors = motors
            .iter()
            .map(|motor| ClearCoreMotor::new(motor.id, motor.scale, tx.clone()))
            .collect();
        let digital_inputs = (0..NO_DIGITAL_INPUTS)
            .map(|index| DigitalInput::new(index as u8, tx.clone()))
            .collect();
        let analog_inputs = (0..NO_ANALOG_INPUTS)
            .map(|index| AnalogInput::new(index as u8 + 3, tx.clone()))
            .collect();
        let outputs = (0..NO_OUTPUTS)
            .map(|index| DigitalOutput::new(index as u8, tx.clone()))
            .collect();

        let h_bridges = [
            HBridge::new(4, 32700, tx.clone()),
            HBridge::new(5, 32700, tx.clone()),
        ];

        Self {
            motors,
            digital_inputs,
            analog_inputs,
            outputs,
            h_bridges,
        }
    }

    pub fn get_motor(&self, id: usize) -> ClearCoreMotor {
        self.motors[id].clone()
    }

    pub fn get_motors(&self) -> Motors {
        self.motors.clone()
    }

    pub fn get_digital_input(&self, id: usize) -> DigitalInput {
        self.digital_inputs[id].clone()
    }

    pub fn get_digital_inputs(&self) -> Inputs {
        self.digital_inputs.clone()
    }

    pub fn get_analog_input(&self, id: usize) -> AnalogInput {
        self.analog_inputs[id].clone()
    }

    pub fn get_analog_inputs(&self) -> AnalogInputs {
        self.analog_inputs.clone()
    }
    pub fn get_output(&self, id: usize) -> DigitalOutput {
        self.outputs[id].clone()
    }

    pub fn get_outputs(&self) -> Outputs {
        self.outputs.clone()
    }

    pub fn get_h_bridge(&self, id: usize) -> HBridge {
        let idx = id - 4;
        self.h_bridges[idx].clone()
    }

    pub fn get_h_bridges(&self) -> HBridges {
        self.h_bridges.clone()
    }
}

pub async fn get_all_motor_states(controller: ControllerHandle) -> Vec<Result<Status, Error>> {
    let mut statuses = Vec::with_capacity(controller.motors.len());
    let mut set = JoinSet::new();
    let motors = controller.get_motors();
    motors.into_iter().for_each(|motor| {
        let motor = motor.clone();
        set.spawn(async move { motor.get_status().await });
    });

    while let Some(result) = set.join_next().await {
        statuses.push(result.unwrap());
    }
    statuses
}

// #[tokio::test]
// async fn test_controller() {
//     let (tx, mut rx) = channel::<Message>(100);
//
//     let motors = [
//         MotorBuilder { id: 0, scale: 800 },
//         MotorBuilder { id: 1, scale: 800 },
//         MotorBuilder { id: 2, scale: 800 },
//         MotorBuilder { id: 3, scale: 800 },
//     ];
//
//     let mock_client = tokio::spawn(async move {
//         if let Some(msg) = rx.recv().await {
//             assert_eq!(*msg.buffer.get(0).unwrap(), 0x02);
//             assert_eq!(*msg.buffer.get(1).unwrap(), b'M');
//             if msg.response.send(msg.buffer).is_err() {
//                 eprintln!("Unable to send Response");
//             }
//         }
//     });
//
//     let controller_task_1 = tokio::spawn(async move {
//         let controller = Controller::new(tx, motors.as_slice());
//
//         let motor0 = controller.get_motor(0);
//         motor0.enable().await.expect("Invalid Message");
//     });
//
//     mock_client.await.unwrap();
//     controller_task_1.await.unwrap();
// }
//
// #[tokio::test]
// async fn test_controller_with_client() {
//     use env_logger::Env;
//     use log::{error, info};
//     use std::net::SocketAddr;
//     use std::sync::atomic::AtomicBool;
//     use std::sync::Arc;
//     use tokio::io::{AsyncReadExt, AsyncWriteExt};
//     use tokio::join;
//     use tokio::net::TcpListener;
//     use tokio::sync::Mutex;
//     use tokio::time::{sleep, Duration};
//
//     env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
//     //We need this MotorBuilder struct to inject the motor scale into the controller, the id part is
//     //Kind of unnecessary, but it might be valuable for having named ids in ryo-os
//     let motors = [
//         MotorBuilder { id: 0, scale: 800 },
//         MotorBuilder { id: 1, scale: 800 },
//         MotorBuilder { id: 2, scale: 800 },
//         MotorBuilder { id: 3, scale: 800 },
//     ];
//
//     let mut reply_buffer = [0; 128];
//
//     let server_task = tokio::spawn(async move {
//         let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
//         let listener = TcpListener::bind(addr).await.unwrap();
//         let (mut stream, _) = listener.accept().await.unwrap();
//         stream.read(reply_buffer.as_mut_slice()).await.unwrap();
//         assert_eq!(reply_buffer[0], 0x02);
//         assert_eq!(reply_buffer[1], b'M');
//         let reply = [2, reply_buffer[1], reply_buffer[2], b'_'];
//         stream.write_all(reply.as_slice()).await.unwrap();
//     });
//     let shutdown = Arc::new(AtomicBool::new(false));
//     signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
//         .expect("Register hook");
//
//     //controller returns its rx that we can use it in its partner client actor, I'm debating whether
//     //Instead of returning a rx we can return a future that can be plugged into spawn directly but
//     let (controller, client) = Controller::with_client("127.0.0.1:8888", motors.as_slice());
//
//     let cc1 = Arc::new(Mutex::from(controller));
//     let task_1_cc_1 = cc1.clone();
//
//     //Tasks that do stuff use a reference to controller
//     let controller_task_1 = tokio::spawn(async move {
//         loop {
//             let motor = task_1_cc_1.lock().await.get_motor(0);
//             if let Err(e) = motor.enable().await {
//                 error!("Motor failed to enable {:?}", e);
//             }
//             sleep(Duration::from_secs(1)).await;
//         }
//     });
//
//     let controller_task_2 = tokio::spawn(async move {
//         loop {
//             {
//                 let input = cc1.lock().await.get_digital_input(0);
//                 info!("Lock Acquired from input task");
//                 let state = input.get_state().await;
//                 info!("{state}");
//             }
//             tokio::time::sleep(Duration::from_secs(1)).await;
//         }
//     });
//
//     //We can start a task with the returned client ensuring that we always use the right client
//     let mock_client = tokio::spawn(client);
//     let _ = join!(
//         mock_client,
//         controller_task_1,
//         controller_task_2,
//         server_task
//     );
// }

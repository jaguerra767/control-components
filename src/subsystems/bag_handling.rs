use crate::components::clear_core_io::{DigitalInput, HBridgeState};
use crate::components::clear_core_motor::{ClearCoreMotor, Status};
use crate::subsystems::linear_actuator::SimpleLinearActuator;
use log::error;
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::time::{interval, sleep};

pub struct BagGripper {
    motor: ClearCoreMotor,
    actuator: SimpleLinearActuator,
    positions: Vec<f64>, //Revs, we have to make a units crate for this
}

impl BagGripper {
    pub fn new(motor: ClearCoreMotor, actuator: SimpleLinearActuator, positions: Vec<f64>) -> Self {
        Self {
            motor,
            actuator,
            positions,
        }
    }

    pub async fn open(&mut self) {
        self.actuator.actuate(HBridgeState::Pos).await;
        sleep(Duration::from_secs_f64(4.0)).await;
    }

    pub async fn close(&mut self) {
        self.actuator.actuate(HBridgeState::Neg).await;
        sleep(Duration::from_secs_f64(4.0)).await;
    }

    pub async fn timed_open(&mut self, time: Duration) {
        self.actuator.actuate(HBridgeState::Pos).await;
        sleep(time).await;
    }

    pub async fn timed_close(&mut self, time: Duration) {
        self.actuator.actuate(HBridgeState::Neg).await;
        sleep(time).await;
    }

    pub async fn rip_bag(&self) -> Result<(), Box<dyn Error>> {
        for pos in self.positions.as_slice() {
            self.motor.absolute_move(*pos).await.unwrap();
            self.motor
                .wait_for_move(Duration::from_millis(150))
                .await
                .unwrap()
        }
        Ok(())
    }
}

pub struct BagDispenser {
    motor: ClearCoreMotor,
    photo_eye: BagSensor,
}

impl BagDispenser {
    pub fn new(motor: ClearCoreMotor, photo_eye_digital_input: DigitalInput) -> Self {
        Self {
            motor,
            photo_eye: BagSensor::new(photo_eye_digital_input),
        }
    }
    pub async fn dispense(&self) -> Result<(), Box<dyn Error>> {
        let mut interval = interval(Duration::from_millis(100));
        self.motor.set_velocity(3.0).await;
        let _ = self.motor.relative_move(100.0).await;
        while !self.photo_eye.photo_eye.get_state().await {
            interval.tick().await;
        }
        self.motor.abrupt_stop().await;
        Ok(())
    }
    pub async fn pull_back(&self) -> Result<(), Box<dyn Error>> {
        let mut interval = interval(Duration::from_millis(100));
        self.motor.set_velocity(1.5).await;
        self.motor.relative_move(-4.6).await.unwrap();
        while self.motor.get_status().await == Status::Moving {
            interval.tick().await;
        }
        Ok(())
    }

    pub async fn check_photo_eye(&self) -> BagSensorState {
        // TODO: i think this may be inverted?
        self.photo_eye.check().await
    }
}

pub enum BagSensorState {
    Bagless,
    Bagful,
}
pub struct BagSensor {
    photo_eye: DigitalInput,
}
impl BagSensor {
    pub fn new(photo_eye: DigitalInput) -> Self {
        Self { photo_eye }
    }

    pub async fn check(&self) -> BagSensorState {
        match self.photo_eye.get_state().await {
            true => BagSensorState::Bagless,
            false => BagSensorState::Bagful,
        }
    }

    pub async fn watcher(photo_eye: DigitalInput, tx: Sender<BagSensorState>) {
        let sensor = Self::new(photo_eye);
        loop {
            tx.clone().send(sensor.check().await).await.unwrap();
            sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn listener(mut rx: Receiver<BagSensorState>) -> Result<(), BagError> {
        while let Some(msg) = rx.recv().await {
            match msg {
                BagSensorState::Bagful => (),
                BagSensorState::Bagless => {
                    error!("Lost Bag");
                    return Err(BagError::LostBag);
                }
            }
        }
        Ok(())
    }

    pub async fn actor(&self) -> Result<(), BagError> {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let photo_eye = self.photo_eye.clone();
        let watcher = tokio::spawn(Self::watcher(photo_eye, tx));
        let listener = tokio::spawn(BagSensor::listener(rx));
        let result = match listener.await.unwrap() {
            Err(e) => Err(e),
            _ => Ok(()),
        };
        watcher.abort();
        result
    }
}

pub enum BagError {
    LostBag,
}

// #[tokio::test]
// async fn test_bag_dispense() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//
//     let bag_dispense_handler = tokio::spawn(async move {
//         let motor = ClearCoreMotor::new(1, 200, tx.clone());
//         motor.enable().await.unwrap();
//         let state = motor.get_status().await.unwrap();
//         assert_eq!(state, Status::Ready);
//         let photo_eye = Input::new(1, tx);
//         let dispenser = BagDispenser::new(motor, photo_eye);
//         dispenser.dispense().await.unwrap();
//         dispenser.pull_back().await.unwrap();
//     });
//
//     let (_, _) = tokio::join!(bag_dispense_handler, cc1_handler);
// }
//
// #[tokio::test]
// async fn test_gripper_motor() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let (tx2, rx2) = tokio::sync::mpsc::channel(10);
//
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
//
//     let motor_handler = tokio::spawn(async move {
//         let motor = ClearCoreMotor::new(2, 200, tx);
//         let gripper = BagGripper::new(
//             motor,
//             SimpleLinearActuator::new(tx2, 4, 0),
//             [0.3, -0.6, 0.3].to_vec(),
//         );
//         gripper.rip_bag().await.unwrap();
//     });
//     let (_, _, _) = tokio::join!(motor_handler, cc1_handler, cc2_handler);
// }
//
// #[tokio::test]
// async fn test_gripper_actuator() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let (tx2, rx2) = tokio::sync::mpsc::channel(10);
//
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
//
//     let actuator_handler = tokio::spawn(async move {
//         let motor = ClearCoreMotor::new(2, 200, tx);
//         let gripper = BagGripper::new(
//             motor,
//             SimpleLinearActuator::new(tx2.clone(), 4, 0),
//             [0.3, -0.6, 0.3].to_vec(),
//         );
//         gripper.open().await.unwrap();
//         tokio::time::sleep(Duration::from_millis(2000)).await;
//         gripper.close().await.unwrap();
//     });
//     let (_, _, _) = tokio::join!(actuator_handler, cc1_handler, cc2_handler);
// }
//
// #[tokio::test]
// async fn test_bag_loading() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let (tx2, rx2) = tokio::sync::mpsc::channel(10);
//
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
//     let task = tokio::spawn(async move {
//         let disp_motor = ClearCoreMotor::new(1, 200, tx.clone());
//         let grip_motor = ClearCoreMotor::new(2, 200, tx.clone());
//         disp_motor.enable().await.unwrap();
//         grip_motor.enable().await.unwrap();
//         tokio::time::sleep(Duration::from_millis(500)).await;
//         let dispenser = BagDispenser::new(disp_motor, Input::new(1, tx.clone()));
//         let gripper = BagGripper::new(
//             grip_motor,
//             SimpleLinearActuator::new(tx2.clone(), 4, 0),
//             [0.4, -0.8, 0.4].to_vec(),
//         );
//         let blower = Output::new(5, tx2);
//         load_bag(dispenser, gripper, blower, /* tokio::sync::mpsc::Sender<GantryCommand> */).await;
//         let gantry = ClearCoreMotor::new(0, 800, tx);
//
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         gantry.relative_move(25.0).await.unwrap();
//     });
//     let (_, _, _) = tokio::join!(task, cc1_handler, cc2_handler);
// }

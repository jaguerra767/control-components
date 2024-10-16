use crate::components::send_recv::SendRecv;
use crate::util::utils::{ascii_to_int, make_prefix, num_to_bytes};
use serde::Serialize;
use std::result::Result;
pub use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::MissedTickBehavior;
use crate::controllers::clear_core::{Message, Error, check_reply};



#[derive(Debug, PartialOrd, PartialEq, Serialize)]
pub enum Status {
    Disabled,
    Enabling,
    Faulted,
    Ready,
    Moving,
}

#[derive(Clone)]
pub struct ClearCoreMotor {
    pub id: u8,
    prefix: [u8; 3],
    scale: usize,
    drive_sender: Sender<Message>,
}

impl ClearCoreMotor {
    pub fn new(id: u8, scale: usize, drive_sender: Sender<Message>) -> Self {
        let prefix = make_prefix(b'M', id);
        ClearCoreMotor {
            id,
            prefix,
            scale,
            drive_sender,
        }
    }

    

    pub async fn enable(&self) -> Result<(), Error> {
        let enable_cmd = [2, b'M', self.id + 48, b'E', b'N', 13];
        let resp = self.write(enable_cmd.as_ref()).await;
        check_reply(&resp).await?;
        let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        while self.get_status().await? == Status::Enabling {
            tick_interval.tick().await;
        }
        if self.get_status().await? == Status::Faulted {
            Err(Error{message: "motor faulted".to_string()})
        } else {
            Ok(()) 
        }
    }

    pub async fn disable(&self) -> Result<(), Error> {
        let enable_cmd = [2, b'M', self.id + 48, b'D', b'E', 13];
        let resp = self.write(enable_cmd.as_ref()).await;
        check_reply(resp.as_ref()).await?;
        Ok(())
    }

    pub async fn absolute_move(&self, position: f64) -> Result<(), Error> {
        let position = num_to_bytes((position * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(position.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"AM");
        msg.extend_from_slice(position.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn relative_move(&self, position: f64) -> Result<(), Error> {
        let position = num_to_bytes((position * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(position.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"RM");
        msg.extend_from_slice(position.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn jog(&self, speed: f64) -> Result<(), Error> {
        let speed = num_to_bytes((speed * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(speed.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"JG");
        msg.extend_from_slice(speed.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn abrupt_stop(&self) -> Result<(), Error> {
        let stop_cmd = [2, b'M', self.id + 48, b'A', b'S', 13];
        let resp = self.write(stop_cmd.as_ref()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Error> {
        let stop_cmd = [2, b'M', self.id + 48, b'S', b'T', 13];
        let resp = self.write(stop_cmd.as_ref()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn set_position(&self, position: isize) -> Result<(), Error> {
        let pos = num_to_bytes(position * self.scale as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(pos.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"SP");
        msg.extend_from_slice(pos.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn set_velocity(&self, mut velocity: f64) -> Result<(), Error> {
        if velocity < 0. {
            velocity = 0.;
        }
        let vel = num_to_bytes((velocity * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(vel.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"SV");
        msg.extend_from_slice(vel.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn set_acceleration(&self, acceleration: f64) -> Result<(), Error> {
        let accel = num_to_bytes((acceleration * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(accel.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"SA");
        msg.extend_from_slice(accel.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn set_deceleration(&self, deceleration: f64) -> Result<(), Error> {
        let accel = num_to_bytes((deceleration * (self.scale as f64)).trunc() as isize);
        let mut msg: Vec<u8> = Vec::with_capacity(accel.len() + self.prefix.len() + 1);
        msg.extend_from_slice(self.prefix.as_slice());
        msg.extend_from_slice(b"SD");
        msg.extend_from_slice(accel.as_slice());
        msg.push(13);
        let resp = self.write(msg.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn get_status(&self) -> Result<Status, Error> {
        let status_cmd = [2, b'M', self.id + 48, b'G', b'S', 13];
        let res = self.write(status_cmd.as_slice()).await;
        match res[3] {
            48 => Ok(Status::Disabled),
            49 => Ok(Status::Enabling),
            50 => Ok(Status::Faulted),
            51 => Ok(Status::Ready),
            52 => Ok(Status::Moving),
            _ => Err(Error{message: "unknown status".to_string()}),
        }
    }

    pub async fn get_position(&self) -> Result<f64, Error> {
        let get_pos_cmd = [2, b'M', self.id + 48, b'G', b'P', 13];
        let res = self.write(get_pos_cmd.as_slice()).await;
        check_reply(&res).await?;
        Ok((ascii_to_int(res.as_slice()) as f64) / (self.scale as f64))
    }

    pub async fn clear_alerts(&self) -> Result<(), Error> {
        let clear_cmd = [2, b'M', self.id + 48, b'C', b'A', 13];
        let resp = self.write(clear_cmd.as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }

    pub async fn wait_for_move(&self, interval: Duration) -> Result<(), Error> {
        let mut tick_interval = tokio::time::interval(interval);
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        while self.get_status().await? == Status::Moving {
            tick_interval.tick().await;
        }
        Ok(())
    }
}

impl SendRecv for ClearCoreMotor {
    fn get_sender(&self) -> &Sender<Message> {
        &self.drive_sender
    }
}

//
// #[tokio::test]
// pub async fn test_motor_enable_disable() {
//     //NOTE: It is UNSAFE to test motion unless we are right in front of Ryo therefore we're only
//     //Testing enable/disable and status in this automated test. For motion, we should test manually
//     let (m1tx, rx) = mpsc::channel::<Message>(100);
//     let m2tx = m1tx.clone();
//     let m3tx = m1tx.clone();
//     let m4tx = m1tx.clone();
//
//     let client = tokio::spawn(client("192.168.1.11:8888", rx));
//
//     let enable = tokio::spawn(async move {
//         let motor1 = AsyncMotor::new(0,800, Controller::new(m1tx));
//         let motor2 = AsyncMotor::new(1,800, Controller::new(m2tx));
//         let motor3 = AsyncMotor::new(2, 800, Controller::new(m3tx));
//         let motor4 = AsyncMotor::new(2, 800, Controller::new(m4tx));
//         motor1.enable().await.expect("No msg received...");
//         motor2.enable().await.expect("No msg received...");
//         motor3.enable().await.expect("No msg received...");
//         motor4.enable().await.expect("No msg received...");
//
//         //Give clear core and ethernet time to enable
//         tokio::time::sleep(Duration::from_millis(1000)).await;
//         //If a motor drive is not connected then Status will return faulted unless HLFB is disabled
//         //on ClearCore
//         let m1_status = motor1.get_status().await.expect("No msg received...");
//         assert_eq!(m1_status, Status::Ready);
//         let m2_status = motor2.get_status().await.expect("No msg received...");
//         assert_eq!(m2_status, Status::Ready);
//         let m3_status = motor3.get_status().await.expect("No msg received...");
//         assert_eq!(m3_status, Status::Ready);
//         let m4_status = motor4.get_status().await.expect("No msg received...");
//         assert_eq!(m4_status, Status::Ready);
//
//         motor1.disable().await.expect("No msg received...");
//         motor2.disable().await.expect("No msg received...");
//         motor3.disable().await.expect("No msg received...");
//         motor4.disable().await.expect("No msg received...");
//
//         let m1_status = motor1.get_status().await.expect("No msg received...");
//         assert_eq!(m1_status, Status::Disabled);
//         let m2_status = motor2.get_status().await.expect("No msg received...");
//         assert_eq!(m2_status, Status::Disabled);
//         let m3_status = motor3.get_status().await.expect("No msg received...");
//         assert_eq!(m3_status, Status::Disabled);
//         let m4_status = motor4.get_status().await.expect("No msg received...");
//         assert_eq!(m4_status, Status::Disabled);
//
//     });
//     client.await.unwrap().expect("TODO: panic message");
//     enable.await.unwrap();
// }
//
// #[tokio::test]
// pub async fn laptop_motor_enable_disable() {
//     //NOTE: It is UNSAFE to test motion unless we are right in front of Ryo therefore we're only
//     //Testing enable/disable and status in this automated test. For motion, we should test manually
//     let (m1tx, rx) = mpsc::channel::<Message>(100);
//     let m2tx = m1tx.clone();
//     let m3tx = m1tx.clone();
//     let m4tx = m1tx.clone();
//
//     let client = tokio::spawn(client("192.168.1.12:8888", rx));
//
//     let enable = tokio::spawn(async move {
//         let motor1 = AsyncMotor::new(0,800, Controller::new(m1tx));
//         let motor2 = AsyncMotor::new(1,800, Controller::new(m2tx));
//         let motor3 = AsyncMotor::new(2, 800, Controller::new(m3tx));
//         let motor4 = AsyncMotor::new(2, 800, Controller::new(m4tx));
//         motor1.enable().await.expect("No msg received...");
//         motor2.enable().await.expect("No msg received...");
//         motor3.enable().await.expect("No msg received...");
//         motor4.enable().await.expect("No msg received...");
//
//         //Give clear core and ethernet time to enable
//         tokio::time::sleep(Duration::from_millis(1000)).await;
//         //If a motor drive is not connected then Status will return faulted unless HLFB is disabled
//         //on ClearCore
//         let m1_status = motor1.get_status().await.expect("No msg received...");
//         assert_eq!(m1_status, Status::Ready);
//         let m2_status = motor2.get_status().await.expect("No msg received...");
//         assert_eq!(m2_status, Status::Ready);
//         let m3_status = motor3.get_status().await.expect("No msg received...");
//         assert_eq!(m3_status, Status::Ready);
//         let m4_status = motor4.get_status().await.expect("No msg received...");
//         assert_eq!(m4_status, Status::Ready);
//
//         motor1.disable().await.expect("No msg received...");
//         motor2.disable().await.expect("No msg received...");
//         motor3.disable().await.expect("No msg received...");
//         motor4.disable().await.expect("No msg received...");
//
//         let m1_status = motor1.get_status().await.expect("No msg received...");
//         assert_eq!(m1_status, Status::Disabled);
//         let m2_status = motor2.get_status().await.expect("No msg received...");
//         assert_eq!(m2_status, Status::Disabled);
//         let m3_status = motor3.get_status().await.expect("No msg received...");
//         assert_eq!(m3_status, Status::Disabled);
//         let m4_status = motor4.get_status().await.expect("No msg received...");
//         assert_eq!(m4_status, Status::Disabled);
//
//     });
//     client.await.unwrap().expect("TODO: panic message");
//     enable.await.unwrap();
// }

// #[tokio::test]
// async fn test_gantry() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let motor = ClearCoreMotor::new(0, 800, tx);
//     let task = tokio::spawn(async move {
//         //motor.enable().await.unwrap();
//         let motor_status = motor.get_status().await.unwrap();
//         assert_eq!(motor_status, Status::Ready);
//         //motor.set_velocity(50.).await.unwrap();
//         motor.relative_move(-22.5).await.unwrap();
//     });
//     let (_, _) = tokio::join!(task, cc1_handler);
// }
//
// #[tokio::test]
// async fn test_gantry_pos() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let motor = ClearCoreMotor::new(0, 800, tx);
//     let task = tokio::spawn(async move {
//         let motor_status = motor.get_status().await.unwrap();
//         assert_eq!(motor_status, Status::Ready);
//         //motor.set_velocity(50.).await.unwrap();
//         //motor.relative_move(-1.0).await.unwrap();
//         let pos = motor.get_position().await.unwrap();
//         println!("{pos}");
//     });
//     let (_, _) = tokio::join!(task, cc1_handler);
// }

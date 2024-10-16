use crate::components::clear_core_io::{AnalogInput, HBridgeState};
use crate::subsystems::linear_actuator::{Output, RelayHBridge};
use log::info;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior};
use crate::controllers::clear_core::Error;

pub struct Hatch {
    actuator: RelayHBridge,
    timeout: Duration,
}

impl Hatch {
    pub fn new(actuator: RelayHBridge, timeout: Duration) -> Self {
        Self { actuator, timeout }
    }

    pub fn from_io(ch_a: Output, ch_b: Output, fb: AnalogInput, timeout: Duration) -> Self {
        Self::new(RelayHBridge::new((ch_a, ch_b), fb), timeout)
    }

    pub async fn get_position(&self) -> Result<isize, Error> {
        self.actuator.get_feedback().await
    }

    pub async fn timed_open(&mut self, time: Duration) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Pos).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await
    }

    pub async fn open(&mut self, set_point: isize) -> Result<(), Error> {
        let star_time = Instant::now();
        let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        self.actuator.actuate(HBridgeState::Pos).await?;
        while self.actuator.get_feedback().await? >= set_point {
            let curr_time = Instant::now();
            if (curr_time - star_time) > self.timeout {
                info!("Timed Out!");
                break;
            }
            tick_interval.tick().await;
        }
        self.actuator.actuate(HBridgeState::Off).await
    }

    pub async fn timed_close(&mut self, time: Duration) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Neg).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await
    }

    pub async fn close(&mut self, set_point: isize) -> Result<(), Error> {
        let star_time = Instant::now();
        self.actuator.actuate(HBridgeState::Neg).await?;
        let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        while self.actuator.get_feedback().await? <= set_point {
            let curr_time = Instant::now();
            if (curr_time - star_time) > self.timeout {
                info!("Timed Out!");
                break;
            }
            tick_interval.tick().await;
        }
        self.actuator.actuate(HBridgeState::Off).await
    }
}

// #[tokio::test]
// async fn open_all() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let (tx2, rx2) = tokio::sync::mpsc::channel(10);
//     let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
//     let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
//     let linear_actuator3 = RelayHBridge::new(tx2.clone(), (0, 1), 3);
//     let linear_actuator4 = RelayHBridge::new(tx2, (2, 3), 4);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
//     let task = tokio::spawn(async move {
//         Hatch::new(linear_actuator1, Duration::from_secs_f64(3.))
//             .timed_open(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator2, Duration::from_secs_f64(3.))
//             .timed_open(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator3, Duration::from_secs_f64(3.))
//             .timed_open(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator4, Duration::from_secs_f64(3.))
//             .timed_open(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//     });
//     let (_, _, _) = tokio::join!(task, cc1_handler, cc2_handler);
// }
//
// #[tokio::test]
// async fn close_all() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let (tx2, rx2) = tokio::sync::mpsc::channel(10);
//     let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
//     let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
//     let linear_actuator3 = RelayHBridge::new(tx2.clone(), (0, 1), 3);
//     let linear_actuator4 = RelayHBridge::new(tx2, (2, 3), 4);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
//     let task = tokio::spawn(async move {
//         Hatch::new(linear_actuator1, Duration::from_secs_f64(3.))
//             .timed_close(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator2, Duration::from_secs_f64(3.))
//             .timed_close(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator3, Duration::from_secs_f64(3.))
//             .timed_close(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//         Hatch::new(linear_actuator4, Duration::from_secs_f64(3.))
//             .timed_close(Duration::from_secs_f64(2.1))
//             .await
//             .unwrap();
//     });
//     let (_, _, _) = tokio::join!(task, cc1_handler, cc2_handler);
// }
//
// #[tokio::test]
// async fn get_all_positions() {
//     let (tx, rx) = tokio::sync::mpsc::channel(10);
//     let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
//     let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
//     let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
//     let task = tokio::spawn(async move {
//         let pos = Hatch::new(linear_actuator1, Duration::from_secs_f64(0.3))
//             .get_position()
//             .await
//             .unwrap();
//         println!("{pos}");
//         let pos = Hatch::new(linear_actuator2, Duration::from_secs_f64(0.3))
//             .get_position()
//             .await
//             .unwrap();
//         println!("{pos}");
//     });
//     let (_, _) = tokio::join!(task, cc1_handler);
// }

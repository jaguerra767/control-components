use crate::components::clear_core_io::HBridgeState;
use crate::interface::tcp::client;
use crate::subsystems::linear_actuator::{LinearActuator, RelayHBridge};
use std::error::Error;
use std::time::Duration;
use tokio::time::Instant;

pub struct Hatch<T: LinearActuator> {
    actuator: T,
    timeout: Duration,
}

impl<T: LinearActuator> Hatch<T> {
    pub fn new(actuator: T, timeout: Duration) -> Self {
        Self { actuator, timeout }
    }

    pub async fn get_position(&self) -> Result<isize, Box<dyn Error>> {
        self.actuator.get_feedback().await
    }

    pub async fn timed_open(&self, time: Duration) -> Result<(), Box<dyn Error>> {
        self.actuator.actuate(HBridgeState::Pos).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await?;
        Ok(())
    }

    pub async fn open(&self, set_point: isize) -> Result<(), Box<dyn Error>> {
        self.actuator.actuate(HBridgeState::Pos).await?;
        let star_time = Instant::now();
        while self.actuator.get_feedback().await? >= set_point {
            let curr_time = Instant::now();
            if (curr_time - star_time) > self.timeout {
                //TODO: Add some proper error handling
                println!("Timed Out!");
                break;
            }
        }
        self.actuator.actuate(HBridgeState::Off).await?;
        Ok(())
    }

    pub async fn timed_close(&self, time: Duration) -> Result<(), Box<dyn Error>> {
        self.actuator.actuate(HBridgeState::Neg).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await?;
        Ok(())
    }

    pub async fn close(&self, set_point: isize) -> Result<(), Box<dyn Error>> {
        self.actuator.actuate(HBridgeState::Neg).await?;
        let star_time = Instant::now();
        while self.actuator.get_feedback().await? <= set_point {
            let curr_time = Instant::now();
            if (curr_time - star_time) > self.timeout {
                //TODO: Add some proper error handling
                println!("Timed Out!");
                break;
            }
        }
        self.actuator.actuate(HBridgeState::Off).await?;
        Ok(())
    }
}

#[tokio::test]
async fn open_all() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (tx2, rx2) = tokio::sync::mpsc::channel(10);
    let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
    let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
    let linear_actuator3 = RelayHBridge::new(tx2.clone(), (0, 1), 3);
    let linear_actuator4 = RelayHBridge::new(tx2, (2, 3), 4);
    let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
    let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
    let task = tokio::spawn(async move {
        Hatch::new(linear_actuator1, Duration::from_secs_f64(3.))
            .timed_open(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator2, Duration::from_secs_f64(3.))
            .timed_open(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator3, Duration::from_secs_f64(3.))
            .timed_open(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator4, Duration::from_secs_f64(3.))
            .timed_open(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
    });
    let (_, _, _) = tokio::join!(task, cc1_handler, cc2_handler);
}

#[tokio::test]
async fn close_all() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (tx2, rx2) = tokio::sync::mpsc::channel(10);
    let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
    let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
    let linear_actuator3 = RelayHBridge::new(tx2.clone(), (0, 1), 3);
    let linear_actuator4 = RelayHBridge::new(tx2, (2, 3), 4);
    let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
    let cc2_handler = tokio::spawn(client("192.168.1.12:8888", rx2));
    let task = tokio::spawn(async move {
        Hatch::new(linear_actuator1, Duration::from_secs_f64(3.))
            .timed_close(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator2, Duration::from_secs_f64(3.))
            .timed_close(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator3, Duration::from_secs_f64(3.))
            .timed_close(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
        Hatch::new(linear_actuator4, Duration::from_secs_f64(3.))
            .timed_close(Duration::from_secs_f64(2.1))
            .await
            .unwrap();
    });
    let (_, _, _) = tokio::join!(task, cc1_handler, cc2_handler);
}

#[tokio::test]
async fn get_all_positions() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let linear_actuator1 = RelayHBridge::new(tx.clone(), (2, 3), 3);
    let linear_actuator2 = RelayHBridge::new(tx, (4, 5), 4);
    let cc1_handler = tokio::spawn(client("192.168.1.11:8888", rx));
    let task = tokio::spawn(async move {
        let pos = Hatch::new(linear_actuator1, Duration::from_secs_f64(0.3))
            .get_position()
            .await
            .unwrap();
        println!("{pos}");
        let pos = Hatch::new(linear_actuator2, Duration::from_secs_f64(0.3))
            .get_position()
            .await
            .unwrap();
        println!("{pos}");
    });
    let (_, _) = tokio::join!(task, cc1_handler);
}

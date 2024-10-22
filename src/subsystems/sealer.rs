use crate::components::clear_core_io::HBridgeState;
use crate::controllers::clear_core::Error;
use crate::subsystems::linear_actuator::{Output, RelayHBridge};
use log::info;
use std::cmp::Ordering;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior};

pub struct Sealer {
    heater: Output,
    actuator: RelayHBridge,
    timeout: Duration,
    extend_setpoint: isize,
    retract_setpoint: isize,
}

impl Sealer {
    pub fn new(
        heater: Output,
        actuator: RelayHBridge,
        timeout: Duration,
        extend_setpoint: isize,
        retract_setpoint: isize,
    ) -> Self {
        Self {
            heater,
            actuator,
            timeout,
            extend_setpoint,
            retract_setpoint,
        }
    }

    async fn get_actuator_position(&mut self) -> Result<isize, Error> {
        self.actuator.get_feedback().await
    }

    async fn absolute_move(&mut self, position: isize) -> Result<(), Error> {
        let current_pos = self.get_actuator_position().await?;
        match current_pos.cmp(&position) {
            Ordering::Greater => self.retract_actuator(position).await,
            Ordering::Less => self.extend_actuator(position).await,
            Ordering::Equal => Ok(()),
        }
    }
    pub async fn timed_extend_actuator(&mut self, time: Duration) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Pos).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await
    }

    async fn extend_actuator(&mut self, set_point: isize) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Pos).await?;
        let star_time = Instant::now();
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

    pub async fn timed_retract_actuator(&mut self, time: Duration) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Neg).await?;
        tokio::time::sleep(time).await;
        self.actuator.actuate(HBridgeState::Off).await
    }

    async fn retract_actuator(&mut self, set_point: isize) -> Result<(), Error> {
        self.actuator.actuate(HBridgeState::Neg).await?;
        let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let star_time = Instant::now();
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

    async fn heat(&mut self, dwell_time: Duration) -> Result<(), Error> {
        self.heater.set_state(true).await?;
        tokio::time::sleep(dwell_time).await;
        self.heater.set_state(false).await
    }

    pub async fn seal(&mut self) -> Result<(), Error> {
        self.absolute_move(self.extend_setpoint).await?;
        self.heat(Duration::from_secs_f64(3.0)).await?;
        self.absolute_move(self.retract_setpoint).await
    }

    pub async fn timed_move_seal(&mut self, time: Duration) -> Result<(), Error> {
        self.timed_extend_actuator(time).await?;
        self.heat(Duration::from_secs_f64(3.)).await?;
        self.timed_retract_actuator(time).await
    }
}

// #[tokio::test]
// async fn test_sealer() {
//     use env_logger::Env;
//     use crate::controllers::clear_core::{MotorBuilder, Controller};
//     use crate::controllers::ek1100_io;
//     env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
//     let interface = "enp1s0f0";
//     let (mut ethercat_io, eth_client) = ek1100_io::Controller::with_client(interface, 1);
//     let eth_client_handler = tokio::spawn(eth_client);
//
//     tokio::time::sleep(Duration::from_secs_f64(3.0)).await;
//     let motors = [MotorBuilder { id: 0, scale: 800 }];
//     let (cc1, cc_client) = Controller::with_client("192.168.1.11:8888", &motors);
//
//     let cc_client_handler = tokio::spawn(cc_client);
//
//     let heater = cc1.get_output(1).unwrap();
//
//     let actuator = ethercat_io.get_io(0).unwrap();
//
//     let extend = 3;
//     let retract = 2;
//
//     let mut sealer = Sealer::new(heater, actuator, extend, retract);
//     tokio::time::sleep(Duration::from_secs_f64(3.0)).await;
//     sealer.seal().await;
//
//     drop(cc1);
//     drop(ethercat_io);
//
//     let _ = tokio::join!(eth_client_handler, cc_client_handler);
// }

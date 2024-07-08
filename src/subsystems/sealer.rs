use crate::components::clear_core_io::DigitalOutput;
use crate::controllers::ek1100_io::IOCard;
use std::time::Duration;
use crate::components::Output;

pub struct Sealer {
    heater: DigitalOutput,
    actuator_io: IOCard,
    extend_id: u8,
    retract_id: u8,
}

impl Sealer {
    pub fn new(
        heater: DigitalOutput,
        actuator_io: IOCard,
        extend_id: u8,
        retract_id: u8,
    ) -> Self {
        Self {
            heater,
            actuator_io,
            extend_id,
            retract_id,
        }
    }

    async fn extend_heater(&mut self) {
        self.actuator_io.set_state(1, self.extend_id, true).await;
        tokio::time::sleep(Duration::from_secs_f64(3.)).await;
        self.actuator_io.set_state(1, self.extend_id, false).await;
    }

    async fn retract_heater(&mut self) {
        self.actuator_io.set_state(1, self.retract_id, true).await;
        tokio::time::sleep(Duration::from_secs_f64(3.)).await;
        self.actuator_io.set_state(1, self.retract_id, false).await;
    }

    async fn heat(&self, dwell_time: Duration) {
        self.heater.set_state(true).await;
        tokio::time::sleep(dwell_time).await;
        self.heater.set_state(false).await;
    }

    pub async fn seal(&mut self) {
        self.extend_heater().await;
        self.heat(Duration::from_secs_f64(3.0)).await;
        self.retract_heater().await;
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

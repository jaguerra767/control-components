use std::error::Error;
use std::time::Duration;
use tokio::time::Instant;
use crate::components::clear_core_io::HBridgeState;
use crate::subsystems::linear_actuator::{LinearActuator};

pub struct Hatch<T: LinearActuator> {
    actuator: T,
    timeout: Duration
}

impl<T: LinearActuator> Hatch<T> {

    pub fn new(actuator: T, timeout: Duration) -> Self {
        Self{actuator ,timeout}
    }
    
    pub async fn open(&self, set_point: isize) -> Result<(), Box<dyn Error>>{
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
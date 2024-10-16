use crate::components::clear_core_motor::ClearCoreMotor;
use crate::controllers::clear_core;
use crate::components::scale::ScaleHandle;
use crate::util::utils::LowPassFilter;
use crate::dispenser::Parameters;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{interval, Instant, MissedTickBehavior};


pub struct DispenseTimeout;
pub enum Error {
    Motor(clear_core::Error),
    Timeout
}

impl From<clear_core::Error> for Error {
    fn from(value: clear_core::Error) -> Self {
       Error::Motor(value)
    }
}



pub struct SetpointDispenser {
    pub node_id: char,
    scale: ScaleHandle,
    motor: ClearCoreMotor,
    parameters: Parameters,
    starting_weight: f64,
}

impl SetpointDispenser {
    pub async fn launch(
        node_id: char,
        motor: ClearCoreMotor,
        scale: ScaleHandle,
        parameters: Parameters,
        sample_time: Duration,
    ) -> Result<Self, Error> {
        motor.enable().await?;
        motor.set_velocity(parameters.motor_speed).await?;
        motor.relative_move(100.).await?;
        let starting_weight = scale
            .get_median_weight(parameters.sample_rate, sample_time)
            .await;
        Ok(Self {
            node_id,
            scale,
            motor,
            parameters,
            starting_weight,
        })
    }

    async fn dispense_complete(&mut self, current_weight:f64, target_weight:f64) -> Result<bool, Error> {
        if current_weight > target_weight + self.parameters.check_offset {
            self.motor.abrupt_stop().await?;
            let current_weight = self.scale.get_median_weight(
                self.parameters.sample_rate, 
                Duration::from_secs_f64(1.0)
            ).await;
            
            if current_weight > target_weight + self.parameters.check_offset {
                return Ok(true);
            }
            self.motor.relative_move(10.).await?;
        }
        Ok(false)
    }


    pub async fn dispense(&mut self, setpoint: f64, timeout: Duration) -> Result<f64, Error> {
        let target_weight = self.starting_weight - setpoint;
        let start_time = Instant::now();
        let mut filter = LowPassFilter::new(
            self.parameters.sample_rate,
            self.parameters.cutoff_frequency,
            self.starting_weight,
        );
        let mut interval = interval(Duration::from_secs_f64(1./self.parameters.sample_rate));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut current_weight = self.scale.get_weight().await;
        let error = Arc::new(Mutex::new((current_weight - target_weight) / setpoint));
        let dispense_complete = Arc::new(AtomicBool::new(false));
        
        //Update motor speed wrt to the error aka P controller
        tokio::spawn({
            let motor = self.motor.clone();
            let speed = self.parameters.motor_speed;
            let error = error.clone();
            let dispense_complete = dispense_complete.clone();
            async move {
                _ = update_motor_speed(error.clone(), dispense_complete, motor, speed).await;
            }
        });
        
        //Actual dispense code
        while !self.dispense_complete(current_weight, target_weight).await? {
            current_weight = filter.apply(self.scale.get_weight().await);
            if Instant::now() - start_time > timeout {
                return Err(Error::Timeout);
            }
            {
                *error.lock().await = (current_weight - target_weight) / setpoint;
            }
            interval.tick().await;
        }
        self.motor.abrupt_stop().await?;
        dispense_complete.store(true, Ordering::Relaxed);
        Ok(current_weight)
    }
}

async fn update_motor_speed(
    error: Arc<Mutex<f64>>,
    dispense_complete: Arc<AtomicBool>,
    motor: ClearCoreMotor,
    base_speed: f64,
) -> Result<(), Error>{
    let mut interval = interval(Duration::from_millis(200));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        if dispense_complete.load(Ordering::Relaxed) {
            break;
        }
        let new_speed = { *error.lock().await * base_speed };
        if new_speed >= 0.1 && new_speed < base_speed {
            motor.set_velocity(new_speed).await?;
        }
        //We need to send a new move command so that the clear core recalculates the new
        //motion profile and actually applies the new velocity
        motor.relative_move(30.).await?;
        interval.tick().await;
    }
    Ok(())
}

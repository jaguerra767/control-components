use std::fmt::Debug;
use log::{error, info};
use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::ScaleCmd;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{Duration, Instant};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    motor_speed: f64,
    sample_rate: f64,
    cutoff_frequency: f64,
    check_offset: f64,
    stop_offset: f64,
}
#[derive(Deserialize, Debug, Clone)]
pub struct WeightedDispense {
    pub setpoint: f64,
    pub timeout: Duration,
}
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Setpoint {
    Weight(WeightedDispense),
    Timed(Duration),
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DispenseParameters {
    pub parameters: Parameters,
    pub setpoint: Setpoint,
}

pub struct Dispenser {
    motor: ClearCoreMotor,
    setpoint: Setpoint,
    parameters: Parameters,
    scale_tx: Sender<ScaleCmd>,
}

impl Dispenser {
    pub fn new(
        motor: ClearCoreMotor,
        setpoint: Setpoint,
        parameters: Parameters,
        scale_tx: Sender<ScaleCmd>
    ) -> Self {
        Self {
            motor,
            setpoint,
            parameters,
            scale_tx,
        }
    }

    async fn get_weight(&self) -> f64 {
        let (rep_tx, rep_rx) = oneshot::channel();
        self.scale_tx.send(ScaleCmd(rep_tx)).await.unwrap();
        rep_rx.await.unwrap()
    }

    async fn get_median_weight(&self, samples: usize, sample_rate: f64) -> f64 {
        let mut buffer = Vec::with_capacity(samples);
        for _ in 0..=samples {
            let weight = self.get_weight().await;
            buffer.push(weight);
            tokio::time::sleep(Duration::from_secs_f64(1./sample_rate)).await;
        }
        buffer.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = buffer.len();
        buffer[middle]
    }

    async fn update_motor_speed(&self, last_cmd_time: Instant, error: f64) -> Option<Instant>{
        let current_time = Instant::now();
        if current_time - last_cmd_time > Duration::from_millis(500) {
            let new_speed = error * self.parameters.motor_speed;
            if new_speed >= 0.1 {
                self.motor.set_velocity(new_speed).await;
            }
            self.motor.relative_move(20.).await.expect("Motor faulted or not enabled");
            Some(Instant::now())
        } else {
            None
        }
    }
    fn at_setpoint(&self, current_weight: f64, target_weight: f64) -> Option<f64> {
        if current_weight < target_weight - self.parameters.check_offset {
            Some(current_weight)
        } else {
            None
        }
    }
    fn check_final_weight(&self, final_weight: Option<f64>, target_weight: f64) -> bool {
        info!("Checking final weight");
        if let Some(weight) = final_weight{
            weight < target_weight - self.parameters.stop_offset 
        } else {
            false
        }
    }
    
    pub async fn dispense(&self, timeout: Duration) {
        //Initialize dispense tracking vars
        let init_time = Instant::now();
        match &self.setpoint {
            Setpoint::Weight(w) => {
                // Set low-pass filter values
                let filter_period = 1./self.parameters.sample_rate;
                let filter_rc = 1./(self.parameters.cutoff_frequency * 2. * std::f64::consts::PI);
                let filter_a = filter_period / (filter_period + filter_rc);
                let filter_b = filter_rc / (filter_period + filter_rc);

                let mut last_sent_motor_cmd = init_time;

                let mut curr_weight = self.get_median_weight(200, self.parameters.sample_rate).await;
                let init_weight = curr_weight;
                let mut final_weight: Option<f64> = None;
                let target_weight = init_weight - w.setpoint;

                //Starting motor moves
                //Run backwards for a bit
                self.motor.set_velocity(self.parameters.motor_speed).await;
                self.motor.relative_move(-10.).await.expect("Motor faulted");
                tokio::time::sleep(Duration::from_secs(3)).await;
                self.motor.relative_move(1000.).await.expect("Motor faulted");
              
                //This while keep going while either final weight is none or while final weight is 
                // not at setpoint
                while self.check_final_weight(final_weight, target_weight) {
                    if self.at_setpoint(curr_weight, target_weight).is_some() {
                        self.motor.abrupt_stop().await;
                        let weight = self.get_median_weight(150, self.parameters.sample_rate).await;
                        final_weight = Some(weight);
                    }
                    let current_time = Instant::now();
                    if current_time - init_time > timeout {
                        self.motor.abrupt_stop().await;
                        error!("Dispense timed out!");
                        break
                    }
                    curr_weight = self.get_weight().await; 
                    curr_weight = filter_a * curr_weight + filter_b * curr_weight;
                    let err = (curr_weight - target_weight)/w.setpoint;
                    if let Some(t) = self.update_motor_speed(last_sent_motor_cmd, err).await{
                        last_sent_motor_cmd = t;
                    }
                }
                let dispensed_weight = final_weight.unwrap();
                info!("Dispensed: {dispensed_weight}");
                
            }
            Setpoint::Timed(d) => {
                self.motor.set_velocity(self.parameters.motor_speed).await;
                self.motor.relative_move(100.).await.expect("Motor faulted");
                tokio::time::sleep(*d).await;
                self.motor.abrupt_stop().await;
            }
        }






    }
}

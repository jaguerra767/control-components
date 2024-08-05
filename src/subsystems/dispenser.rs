use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::ScaleCmd;
use log::{error, info};
use serde::Deserialize;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, Instant};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    pub motor_speed: f64,
    pub sample_rate: f64,
    pub cutoff_frequency: f64,
    pub check_offset: f64,
    pub stop_offset: f64,
    pub retract_before: Option<f64>,
    pub retract_after: Option<f64>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            motor_speed: 0.3,
            sample_rate: 50.0,
            cutoff_frequency: 0.5,
            check_offset: 15.0,
            stop_offset: 7.0,
            retract_before: None,
            retract_after: None,
        }
    }
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
#[derive(Deserialize, Debug, Clone)]
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
        scale_tx: Sender<ScaleCmd>,
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
            tokio::time::sleep(Duration::from_secs_f64(1. / sample_rate)).await;
        }
        buffer.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = buffer.len() / 2;
        buffer[middle]
    }

    async fn update_motor_speed(&self, last_cmd_time: Instant, error: f64) -> Option<Instant> {
        let current_time = Instant::now();
        if current_time - last_cmd_time > Duration::from_millis(200) {
            let new_speed = error * self.parameters.motor_speed;
            if new_speed >= 0.1 {
                self.motor
                    .set_velocity(if new_speed > self.parameters.motor_speed {
                        self.parameters.motor_speed
                    } else {
                        new_speed
                    })
                    .await;
            }
            self.motor
                .relative_move(20.)
                .await
                .expect("Motor faulted or not enabled");
            Some(Instant::now())
        } else {
            None
        }
    }
    
    async fn retract_before(&self) {
        if let Some(retract) = self.parameters.retract_before {
            self.motor
                .relative_move(-retract)
                .await
                .expect("Motor faulted");
            self.motor
                .wait_for_move(Duration::from_millis(50))
                .await
                .unwrap();
        }
    }
    
    async fn retract_after(&self) {
        if let Some(retract) = self.parameters.retract_after {
            self.motor
                .relative_move(-retract)
                .await
                .expect("Motor faulted");
        }
    }

    pub async fn dispense(&self, timeout: Duration) {
        let init_time = Instant::now();
        match &self.setpoint {
            Setpoint::Weight(w) => {
                let mut interval = interval(Duration::from_millis(500));
                
                // Set low-pass filter values
                let filter_period = 1. / self.parameters.sample_rate;
                let filter_rc = 1. / (self.parameters.cutoff_frequency * 2. * std::f64::consts::PI);
                let filter_a = filter_period / (filter_period + filter_rc);
                let filter_b = filter_rc / (filter_period + filter_rc);

                let mut last_sent_motor_cmd = init_time;

                let mut curr_weight = self
                    .get_median_weight(100, self.parameters.sample_rate)
                    .await;
                let init_weight = curr_weight;
                let target_weight = init_weight - w.setpoint;

                // Starting motor moves
                self.motor.set_velocity(self.parameters.motor_speed).await;
                self.retract_before().await;
                self.motor.relative_move(100.).await.expect("Motor faulted");

                let shutdown = Arc::new(AtomicBool::new(false));
                signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
                    .expect("Register hook");
                //This while keep going while either final weight is none or while final weight is
                // not at setpoint
                let end_condition = loop {
                    if shutdown.load(Ordering::Relaxed) {
                        self.motor.abrupt_stop().await;
                        break DispenseEndCondition::Failed;
                    }

                    let current_time = Instant::now();
                    if current_time - init_time > timeout {
                        self.motor.abrupt_stop().await;
                        error!("Dispense timed out!");
                        break DispenseEndCondition::Timeout(init_weight - curr_weight);
                    }
                    curr_weight = filter_a * self.get_weight().await + filter_b * curr_weight;
                    let err = (curr_weight - target_weight) / w.setpoint;
                    if let Some(t) = self.update_motor_speed(last_sent_motor_cmd, err).await {
                        last_sent_motor_cmd = t;
                    }

                    if curr_weight < target_weight + self.parameters.check_offset {
                        info!("Check offset reached");
                        self.motor.abrupt_stop().await;
                        let check_weight = self
                            .get_median_weight(15, self.parameters.sample_rate)
                            .await;
                        if check_weight < target_weight + self.parameters.stop_offset {
                            self.retract_after().await;
                            break DispenseEndCondition::WeightAchieved(init_weight - check_weight);
                        }
                        self.motor.relative_move(10.).await.unwrap();
                    }
                    interval.tick().await;
                };
                self.motor.abrupt_stop().await;
                info!("End Condition: {:?}", end_condition);
            }
            Setpoint::Timed(d) => {
                self.motor.set_velocity(self.parameters.motor_speed).await;
                self.retract_before().await;
                self.motor.relative_move(100.).await.expect("Motor faulted");
                tokio::time::sleep(*d).await;
                self.motor.abrupt_stop().await;
                self.retract_after().await;
            }
        }
    }
}
#[derive(Debug)]
pub enum DispenseEndCondition {
    Timeout(f64),
    WeightAchieved(f64),
    Failed,
}

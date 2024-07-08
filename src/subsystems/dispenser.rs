use tokio::time::Duration;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::ScaleCmd;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Parameters{
    motor_speed: f64,
    sample_rate: f64,
    cutoff_frequency: f64,
    check_offset: f64,
    stop_offset: f64
}
#[derive(Deserialize, Debug)]
pub struct WeightedDispense{
    setpoint: f64,
    timeout: Duration
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Setpoint{
    Weight(WeightedDispense),
    Timed(Duration)
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DispenseParameters {
    parameters: Parameters,
    setpoint: Setpoint
}

pub struct Dispenser {
    motor: ClearCoreMotor,
    parameters: Parameters,
    scale_tx: Sender<ScaleCmd>
}

pub async fn dispense(motor: ClearCoreMotor, parameters: Parameters, scale_tx: Sender<ScaleCmd>) {
    
}

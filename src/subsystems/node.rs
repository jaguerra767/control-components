use tokio::join;

use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::Scale;
use tokio::time::{Duration, Instant};
use crate::interface::tcp::client;

pub struct DispensingParameters  {
    serving_weight: f64,
    motor_speed: f64,
    sample_rate: f64,
    cutoff_frequency: f64,
    check_offset: f64,
    stop_offset: f64,
} impl DispensingParameters {
    pub fn new(serving_weight: f64,
               motor_speed: f64,
               sample_rate: f64,
               cutoff_frequency: f64,
               check_offset: f64,
               stop_offset: f64,
    ) -> Self {
        Self {
            serving_weight,
            motor_speed,
            sample_rate,
            cutoff_frequency,
            check_offset,
            stop_offset
        }
    }
}

pub struct Node {
    motor: ClearCoreMotor,
}

impl Node {
  
    pub fn new(motor: ClearCoreMotor) -> Self {
        Self { motor }
    }

    pub async fn connect_scale(&self, mut scale: Scale) -> Scale {
        tokio::task::spawn_blocking( move || {
            Scale::connect(scale).expect("Scale failed to connect")
        }).await.unwrap()
    }

    pub async fn read_scale(&self, scale: Scale) -> (Scale, f64) {
        tokio::task::spawn_blocking( move || {
            Scale::live_weigh(scale).expect("Scale failed to weigh")
        }).await.unwrap()
    }

    pub async fn read_scale_median(&self, scale: Scale, time: Duration, sample_rate: usize) -> (Scale, f64) {
        tokio::task::spawn_blocking( move || {
            Scale::weight_by_median(scale, time, sample_rate).expect("Failed to weigh scale")
        }).await.unwrap()
    }


    pub async fn dispense(&self,
                          scale: Scale,
                          parameters: DispensingParameters
                          // serving: f64,
                          // sample_rate: f64,
                          // cutoff_frequency: f64,
                          // motor_speed: f64,
    ) -> (Scale, Vec<Duration>, Vec<f64>) {
        // Prime conveyor
        self.motor.set_velocity(2.*parameters.motor_speed).await.unwrap();
        self.motor.relative_move(-10000.).await.unwrap();

        // Set LP filter values
        let filter_period = 1. / parameters.sample_rate;
        let filter_rc = 1. / (parameters.cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
        
        let (mut scale, init_weight) = self.read_scale_median(scale, Duration::from_secs(3), 50).await;

        let mut curr_weight = init_weight;
        let target_weight = init_weight - parameters.serving_weight;
        let mut reading: f64;
        let mut final_weight: f64;
        
        let timeout = Duration::from_secs(90);
        let send_command_delay = Duration::from_millis(500);

        let mut times: Vec<Duration> = Vec::new();
        let mut weights: Vec<f64> = Vec::new();

        self.motor.set_velocity(parameters.motor_speed).await.expect("Failed to change velocity");
        self.motor.relative_move(10000.).await.expect("Failed to send move command");
        let (scale, dispensed) = loop {
            if curr_weight < target_weight - parameters.check_offset {
                self.motor.abrupt_stop().await.expect("Failed to stop");
                (scale, final_weight) = self.read_scale_median(scale, Duration::from_secs(2), 50).await;
                if final_weight <= target_weight - parameters.stop_offset {
                    break (scale, init_weight - final_weight)
                }
            }
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                // TODO: maybe violently run in reverse for a couple seconds and let it keep running?
                self.motor.abrupt_stop().await.expect("Failed to stop");
                println!("WARNING: Dispense timed out!");
                break (scale, init_weight-curr_weight)
            }
            (scale, reading) = self.read_scale(scale).await;
            curr_weight = filter_a * reading + filter_b * curr_weight;

            times.push(curr_time-init_time);
            weights.push(reading);

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                let err = (curr_weight - target_weight) / parameters.serving_weight;
                let new_motor_speed = err * parameters.motor_speed;
                if new_motor_speed >= 0.1 {
                    self.motor.set_velocity(new_motor_speed).await.expect("Failed to change speed");
                }
                self.motor.relative_move(10000.0).await.expect("Failed to update");
            }
        };
        println!("Dispensed: {:.1} g", dispensed);
        (scale, times, weights)
    }
//
    pub async fn timed_dispense(&self,
                                scale: Scale,
                                dispense_time: Duration,
                                sample_rate: f64,
                                cutoff_frequency: f64,
                                motor_speed: f64,
    // ) -> (Scale, Vec<Duration>, Vec<f64>) {
       ) -> Scale {
        // Set LP filter values
        let filter_period = 1. / sample_rate;
        let filter_rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
         
        let (mut scale, init_weight) = self.read_scale_median(scale, Duration::from_secs(3), 200).await;

        let mut curr_weight = init_weight;
        let mut reading: f64;
        let timeout = dispense_time;
        let send_command_delay = Duration::from_millis(250);

        // Data tracking
        let mut times = Vec::new();
        let mut weights = Vec::new();
        self.motor.set_velocity(motor_speed).await.expect("TODO: panic message");
        self.motor.relative_move(10000.0).await.expect("Failed to update");
        loop {
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                self.motor.abrupt_stop().await.expect("Failed to stop");
                break;
            }
            // curr_weight = filter_a * self.read_scale(scale).expect("Failed to weigh scale") + filter_b * curr_weight;
            (scale, reading) = self.read_scale(scale).await;
            curr_weight = filter_a * reading + filter_b * curr_weight;

            times.push(curr_time-init_time);
            weights.push(curr_weight);

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                self.motor.relative_move(10000.0).await.expect("Failed to update");
            }
        }

        // let final_weight = self.scale.weight_by_median(500, 100).expect("Failed to weigh scale");
        let (scale, final_weight) = self.read_scale_median(scale, Duration::from_secs(3), 200).await;
        println!("Dispensed: {:.1} g", init_weight-final_weight);
        // (scale, times, weights)
        scale    
}
}


#[tokio::test]
async fn test() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let client = tokio::spawn(client("192.168.1.12:8888", rx));
    ClearCoreMotor::new(3, 800, tx.clone()).enable().await.unwrap();
    let node = Node::new(ClearCoreMotor::new(3, 800, tx));
    let mut scale = node.connect_scale(Scale::new(716709)).await;
    scale = Scale::change_coefficients(scale, vec![4780449.913365008, 2596299.373482612, -4975764.006916862, 4998589.065848139]);
    let task = tokio::spawn(async move {
        // node.dispense(scale, 60., 50., 0.5, 0.3).await
    });
    let (_, _) = join!(client, task);
    println!("DEBUG: Complete!")
}

#[tokio::test]
async fn motor_test() {
    tokio::time::sleep(Duration::from_secs(2)).await;
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let client = tokio::spawn(client("192.168.1.12:8888", rx));
    ClearCoreMotor::new(0, 800, tx.clone()).enable().await.unwrap();
    let task = tokio::spawn( async move {
        let motor = ClearCoreMotor::new(0, 800, tx);
        motor.set_velocity(0.3).await.unwrap();
        motor.relative_move(100000.0).await.unwrap();
        for _ in 0..10 {
            motor.relative_move(100000.0).await.unwrap();
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        motor.abrupt_stop().await.unwrap();
    });
    let (_, _) = join!(client, task);
    println!("DEBUG: Complete!")
}


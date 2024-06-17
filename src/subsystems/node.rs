
use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::Scale;
use tokio::time::{Duration, Instant};
use crate::interface::tcp::client;

pub struct Node {
    scale: Scale,
    motor: ClearCoreMotor,
}

impl Node {
    pub fn new(scale: Scale, motor: ClearCoreMotor) -> Self {
        Self {scale, motor }
    }

    pub fn connect(& mut self) {
        self.scale.connect().expect("Scale failed to connect");
    }
    

    pub async fn dispense(&self,
                          serving: f64,
                          sample_rate: f64,
                          cutoff_frequency: f64,
                          motor_speed: isize,
    ) {
        // Set LP filter values
        let filter_period = 1. / sample_rate;
        let filter_rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
        
        
        
        let init_weight = self.scale.weight_by_median(500, 100)
            .expect("Failed to weigh scale");
        let mut curr_weight = init_weight;
        let target_weight = init_weight - serving;
        let timeout = Duration::from_secs(90);
        let send_command_delay = Duration::from_millis(250);

        while curr_weight < target_weight {
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                println!("WARNING: Dispense timed out!");
                break;
            }
            curr_weight = filter_a * self.scale.live_weigh().expect("Failed to weigh scale") + filter_b * curr_weight;

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                let err = (curr_weight - target_weight) / serving;
                let new_motor_speed = err * (motor_speed as f64);
                self.motor.set_velocity(new_motor_speed).await.expect("Failed to change speed");
                self.motor.relative_move(1000.0).await.expect("Failed to update");
            }
        }

        let final_weight = self.scale.weight_by_median(500, 100).expect("Failed to weigh scale");
        println!("Dispensed: {:.1} g", final_weight);
    }

    pub async fn timed_dispense(&self,
                               dispense_time: Duration,
                               sample_rate: f64,
                               cutoff_frequency: f64,
    ) -> (Vec<Duration>, Vec<f64>) {
        // Set LP filter values
        let filter_period = 1. / sample_rate;
        let filter_rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
        let init_weight = self.scale.weight_by_median(500, 100)
            .expect("Failed to weigh scale");
        
        
        let mut curr_weight = init_weight;
        let timeout = dispense_time;
        let send_command_delay = Duration::from_millis(250);

        // Data tracking
        let mut times = Vec::new();
        let mut weights = Vec::new();
        self.motor.set_velocity(1.0).await.expect("TODO: panic message");
        self.motor.relative_move(1000.0).await.expect("Failed to update");
        loop {
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                break;
            }
            curr_weight = filter_a * self.scale.live_weigh().expect("Failed to weigh scale") + filter_b * curr_weight;

            times.push(curr_time-init_time);
            weights.push(curr_weight);

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                self.motor.relative_move(1000.0).await.expect("Failed to update");
            }
        }

        let final_weight = self.scale.weight_by_median(500, 100).expect("Failed to weigh scale");
        println!("Dispensed: {:.1} g", final_weight);
        (times, weights)
    }
}


async fn connect_scale(mut scale: Scale) -> Scale{
    let task = tokio::task::spawn_blocking(move ||{
        scale.connect().unwrap();
        scale
    });
    task.await.unwrap()
}

async fn read_scale(scale: Scale) -> (f64, Scale) {
    let task = tokio::task::spawn_blocking(move || {
        (scale.weight_by_median(50,200).unwrap(), scale)
    });
    task.await.unwrap()
}

async fn alt_dispense(motor: ClearCoreMotor) {
    let mut scale = connect_scale(Scale::new(716709)).await;
    let mut weight = 0.0;
    println!("{weight}");
    motor.set_velocity(0.5).await.unwrap();
    println!("Set Velocity command sent!");
    motor.relative_move(1000.0).await.unwrap();
    println!("Move Command Sent");
    let mut current_time =  Instant::now();
    let start_time = current_time;
    (weight, scale) = read_scale(scale).await;
    println!("Starting: {weight}");
    loop {
        (weight, scale) = read_scale(scale).await;
        current_time = Instant::now();
        if (current_time - start_time) > Duration::from_secs(10) {
            motor.abrupt_stop().await.unwrap();
            break
        }
    }
    println!("Ending: {weight}");
}
#[tokio::test]
async fn test_node() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let client = tokio::spawn(client("192.168.1.12:8888", rx));
    let task = tokio::spawn(alt_dispense(ClearCoreMotor::new(0, 800, tx)));
    task.await.unwrap();
    let _ = client.await.unwrap();
}


use crate::components::load_cell::LoadCell;
use crate::util::utils::{dot_product, median};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use std::{array, thread};

struct Scale {
    phidget_id: i32,
    receiver: Receiver<ScaleMessage>,
    cells: [LoadCell; 4],
    cell_coefficients: [f64; 4],
    tare_offset: f64,
}

enum ScaleMessage {
    UpdateCoefficients([f64; 4]),
    GetWeight {
        reply: Sender<f64>,
    },
    GetMedianWeight {
        sample_rate: f64,
        time: Duration,
        reply: Sender<f64>,
    },
    GetMedianWeights {
        sample_rate: f64,
        time: Duration,
        reply: Sender<Vec<f64>>,
    },
}

impl Scale {
    fn new(phidget_id: i32, receiver: Receiver<ScaleMessage>) -> Self {
        let cells: [LoadCell; 4] = array::from_fn(|i| LoadCell::new(phidget_id, i as i32));
        let cell_coefficients: [f64; 4] = [0.0; 4];
        Self {
            phidget_id,
            receiver,
            cells,
            cell_coefficients,
            tare_offset: 0.,
        }
    }

    fn connect(&mut self) {
        for lc in &mut self.cells {
            lc.connect().unwrap_or_else(|_| {
                panic!(
                    "Failed to connect to load cell in phidget: {}",
                    self.phidget_id
                )
            });
        }
    }

    fn update_coefficients(&mut self, coefficients: [f64; 4]) {
        self.cell_coefficients = coefficients;
    }
    fn get_readings(&self) -> Vec<f64> {
        // Gets each load cell reading from Phidget
        // and returns them in a matrix.
        self.cells
            .as_slice()
            .iter()
            .map(|cell| cell.get_reading().expect("Failed to get reading"))
            .collect()
    }

    fn get_weight(&self) -> f64 {
        // Gets the instantaneous weight measurement
        let readings = self.get_readings();
        dot_product(readings.as_slice(), self.cell_coefficients.as_slice()) - self.tare_offset
    }

    fn get_median_weight(&self, sample_rate: f64, time: Duration) -> f64 {
        let samples = (sample_rate * time.as_secs_f64()) as usize;
        let interval = Duration::from_secs_f64(1. / sample_rate);
        let mut weights = Vec::with_capacity(samples);
        let mut last_cycle_time = Instant::now();
        while weights.len() < samples {
            let current_time = Instant::now();
            if (current_time - last_cycle_time) > interval {
                let weight = self.get_weight();
                weights.push(weight);
                last_cycle_time = current_time;
            }
        }
        median(&mut weights)
    }

    fn get_median_weights(&self, sample_rate: f64, time: Duration) -> Vec<f64> {
        let mut samples = (sample_rate * time.as_secs_f64()) as usize;
        let interval = Duration::from_secs_f64(1. / sample_rate);
        let mut readings: Vec<Vec<f64>> = vec![vec![0.; samples]; 4];
        let mut medians = vec![0.; 4];
        let mut last_cycle_time = Instant::now();
        while samples > 0 {
            let current_time = Instant::now();
            if (current_time - last_cycle_time) > interval {
                for (idx, cell) in self.cells.iter().enumerate().take(self.cells.len()) {
                    readings[idx].push(cell.get_reading().expect("Failed to get reading"))
                }
                for cell in 0..self.cells.len() {
                    medians[cell] = median(&mut readings[cell]);
                }
                last_cycle_time = current_time;
                samples -= 1;
            }
        }
        medians
    }

    fn handle_message(&mut self, message: ScaleMessage) {
        match message {
            ScaleMessage::UpdateCoefficients(coefficients) => {
                self.update_coefficients(coefficients)
            }
            ScaleMessage::GetWeight { reply } => {
                let weight = self.get_weight();
                reply.send(weight).unwrap();
            }
            ScaleMessage::GetMedianWeight {
                sample_rate,
                time,
                reply,
            } => {
                let weight = self.get_median_weight(sample_rate, time);
                reply.send(weight).unwrap();
            }
            ScaleMessage::GetMedianWeights {
                sample_rate,
                time,
                reply,
            } => {
                let weights = self.get_median_weights(sample_rate, time);
                reply.send(weights).unwrap();
            }
        }
    }
}

fn run_scale(mut scale: Scale) {
    scale.connect();
    while let Ok(message) = scale.receiver.recv() {
        scale.handle_message(message);
    }
}

#[derive(Clone)]
pub struct ScaleHandle {
    sender: Sender<ScaleMessage>,
}

impl ScaleHandle {
    pub fn new(phidget_id: i32) -> Self {
        let (req_tx, req_rx) = channel();
        let scale = Scale::new(phidget_id, req_rx);
        thread::spawn(move || run_scale(scale));
        Self { sender: req_tx }
    }

    pub fn update_coefficients(&mut self, coefficients: [f64; 4]) {
        self.sender
            .send(ScaleMessage::UpdateCoefficients(coefficients))
            .unwrap()
    }

    pub async fn get_weight(&self) -> f64 {
        let (resp_tx, resp_rx) = channel();
        let msg = ScaleMessage::GetWeight { reply: resp_tx };
        self.sender.send(msg).unwrap();
        tokio::task::spawn_blocking(move || resp_rx.recv().unwrap())
            .await
            .unwrap()
    }

    pub async fn get_median_weight(&self, sample_rate: f64, time: Duration) -> f64 {
        let (resp_tx, resp_rx) = channel();
        let msg = ScaleMessage::GetMedianWeight {
            sample_rate,
            time,
            reply: resp_tx,
        };
        self.sender.send(msg).unwrap();
        tokio::task::spawn_blocking(move || resp_rx.recv().unwrap())
            .await
            .unwrap()
    }

    pub async fn get_median_weights(&self, sample_rate: f64, time: Duration) -> Vec<f64> {
        let (resp_tx, resp_rx) = channel();
        let msg = ScaleMessage::GetMedianWeights {
            sample_rate,
            time,
            reply: resp_tx,
        };
        self.sender.send(msg).unwrap();
        tokio::task::spawn_blocking(move || resp_rx.recv().unwrap())
            .await
            .unwrap()
    }
}

// #[test]
// fn calibrate() {
//     let mut scale = Scale::new(716623);
//     scale.connect();
//     let readings = scale.get_median_weights(50., Duration::from_secs(1));
//     println!("Cell Medians: {:?}", readings)
// }
//
// #[test]
// fn connect_scale_cells() -> Result<(), Box<dyn Error>> {
//     let scale = Scale::new(716709);
//     scale.connect();
//     Ok(())
// }
//
// #[test]
// fn read_scale() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716709);
//     scale.connect();
//     let readings = scale.get_readings();
//     println!("Scale Readings: {:?}", readings);
//     Ok(())
// }
//
// #[test]
// fn live_weigh_scale() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716709);
//     scale.connect();
//     let weight = scale.live_weigh();
//     println!("Weight: {:?}", weight);
//
//     Ok(())
// }
//
// #[test]
// fn weigh_scale() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716623);
//     scale.connect();
//     let coefficients = [
//         4780449.913365008,
//         2596299.373482612,
//         -4975764.006916862,
//         4998589.065848139,
//     ];
//     scale.update_coefficients(coefficients);
//     let weight = scale.get_median_weight(50., Duration::from_secs(3));
//     println!("Weight: {:?}", weight - 4268.);
//     Ok(())
// }

use std::{io};
use std::error::Error;
use std::thread::sleep;
use linalg::{LinearSystem, MatrixError};
use tokio::time::{Duration, Instant};
use crate::components::load_cell::LoadCell;


pub struct Scale {
    cells: [LoadCell; 4],
    cell_coefficients: Vec<f64>,
    tare_offset: f64,

}

impl Scale {
    pub fn new(phidget_id: i32) -> Self {
        let cells = [
            LoadCell::new(phidget_id, 0),
            LoadCell::new(phidget_id, 1),
            LoadCell::new(phidget_id, 2),
            LoadCell::new(phidget_id, 3)
        ];
        
        // Self { cells, cell_coefficients: vec![vec![1.]; 4], tare_offset: 0. }
        Self { cells, cell_coefficients: vec![1.; 4], tare_offset: 0. }
    }

    pub fn connect(mut scale: Self) -> Result<Self, Box<dyn Error>> {
        for cell in 0..scale.cells.len() {
            scale.cells[cell].connect()?;
        }
        Ok(scale)
    }
    
    fn get_readings(scale: Self) -> Result<(Self, Vec<f64>), Box<dyn Error>> {
        // Gets each load cell reading from Phidget
        // and returns them in a matrix.

        let mut readings = vec![0.; 4];
        for cell in 0..scale.cells.len() {
            readings[cell] = scale.cells[cell].get_reading()?;
        }
        Ok((scale, readings))
    }

    pub fn live_weigh(mut scale: Self) -> Result<(Self, f64),  Box<dyn Error>> {
        // Gets the instantaneous weight measurement
        // from the scale by taking the sum of each
        // load cell's reading, weighted by its
        // coefficient.
        let readings: Vec<f64>;
        (scale, readings) = Scale::get_readings(scale)?;
        let weight = dot(readings, scale.cell_coefficients.clone()) - scale.tare_offset.clone();
        Ok((scale, weight))

    }

    pub fn weight_by_median(mut scale: Self, time: Duration, sample_rate: usize) -> Result<(Self, f64), Box<dyn Error>> {
        let mut weights = Vec::new();
        let delay = Duration::from_secs_f64(1. / sample_rate as f64);
        let start_time = Instant::now();
        scale = loop {
            if Instant::now() - start_time > time {
                break scale
            }
            let weight: f64;
            (scale, weight) = Scale::live_weigh(scale)?;
            weights.push(weight);
            sleep(delay);
        };
        Ok((scale, Scale::median(&mut weights)))
    }

    fn median(weights: &mut Vec<f64>) -> f64 {
        weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = weights.len() / 2;
        weights[middle]

    }

    // fn get_medians(mut scale: Self, time: Duration, sample_rate: usize) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    //     let mut readings: Vec<Vec<f64>> = vec![vec![]; 4];
    //     let mut medians = vec![0.; 4];
    //     let delay = Duration::from_millis(1000/sample_rate as u64);
    //     let _start_time = Instant::now();
    //     for _sample in 0..samples {
    //         for cell in 0..self.cells.len() {
    //             readings[cell].push(self.cells[cell].get_reading()?);
    //         }
    //         sleep(delay);
    //     }
    //     for cell in 0..self.cells.len() {
    //         medians[cell] = Scale::median(&mut readings[cell]);
    //     }
    //
    //     Ok(vec![medians])
    // }


    // pub fn tare(mut self, time: Duration, sample_rate: usize) -> Result<(), Box<dyn Error>> {
    //     let (self, resting_weight) = self.weight_by_median(time, sample_rate)?;
    //     self.tare_offset = self.tare_offset.clone() + resting_weight;
    //     Ok(())
    // }

//
//     pub fn calibrate(&mut self, test_mass: f64, samples: usize, sample_rate: usize) -> Result<(), Box<dyn Error>> {
//         let mut trial_readings = vec![vec![0.; self.cells.len()]; self.cells.len()];
//         let test_mass_vector = vec![vec![test_mass]; self.cells.len()];
//         for trial in 0..self.cells.len() {
//             println!("Place/move test mass and press key");
//             let mut input = String::new();
//             let _user_input = io::stdin().read_line(&mut input);
//             println!("Weighing...");
//             let readings = self.get_medians(samples, sample_rate)?;
//             trial_readings[trial].clone_from(&LinearSystem::transpose(&readings)[0]);
//         }
//         println!("DEBUG: {:?}, {:?}", trial_readings, test_mass_vector);
//         let mut system = LinearSystem::new(trial_readings, test_mass_vector)?;
//         system.display();
//         self.change_coefficients(system.solve()?);
//
//         Ok(())
//     }
//
    pub fn change_coefficients(mut scale: Self, coefficients: Vec<f64>) -> Self {
        scale.cell_coefficients = coefficients;
        scale
    }

    pub fn diagnose(mut scale: Self, duration: Duration, sample_rate: usize) -> Result<(Self, Vec<Duration>, Vec<f64>), Box<dyn Error>> {
        let mut times = Vec::new();
        let mut weights = Vec::new();
        let data_interval = Duration::from_secs_f64(1. / sample_rate as f64);
        let init_time = Instant::now();

        scale = loop {
            if Instant::now() - init_time > duration {
                break scale
            }
            let weight: f64;
            (scale, weight) = Scale::live_weigh(scale)?;
            let time = Instant::now() - init_time;
            times.push(time);
            weights.push(weight);
            sleep(data_interval);
        };

        Ok((scale, times, weights))
    }

}

fn dot(vec1: Vec<f64>, vec2: Vec<f64>) -> f64 {
    assert_eq!(vec1.len(), vec2.len());
    let mut sum = 0.;
    for elem in 0..vec1.len() {
        sum += vec1[elem]*vec2[elem];
    }
    sum
}


#[derive(Debug)]
pub enum ScaleError {
    LoadCellError,
    MatrixError(MatrixError),
    IoError(io::Error),
}


#[test]
fn connect_scale_cells() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709);
    scale = Scale::connect(scale)?;
    Ok(())
}

#[test]
fn read_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709);
    scale = Scale::connect(scale)?;
    let (_scale, readings) = Scale::get_readings(scale)?;
    println!("Scale Readings: {:?}", readings);
    Ok(())
}

#[test]
fn live_weigh_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709);
    scale = Scale::connect(scale)?;
    let (scale, weight) = Scale::live_weigh(scale)?;
    println!("Weight: {:?}", weight);

    Ok(())
}

#[test]
fn weigh_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709);
    scale = Scale::connect(scale)?;
    let (scale, weight) = Scale::weight_by_median(scale, Duration::from_secs(1), 100)?;
    println!("Weight: {:?}", weight);

    Ok(())
}
//
// #[test]
// fn calibrate_scale() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716709);
//     scale.connect()?;
//     scale.calibrate(437.7, 1000, 100)?;
//
//     Ok(())
// }
//
// #[test]
// fn get_medians() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716709);
//     scale.connect()?;
//     let medians = scale.get_medians(1000, 50)?;
//     println!("Medians: {:?}", medians);
//     Ok(())
// }
//
// #[test]
// fn diagnose_scale() -> Result<(), Box<dyn Error>> {
//     let mut scale = Scale::new(716709);
//     scale.connect()?;
//     let (_times, _weights) = scale.diagnose(Duration::from_secs(5), 100)?;
//     Ok(())
// }

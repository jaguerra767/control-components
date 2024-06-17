use std::{time, io};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use linalg::{LinearSystem, MatrixError};
use tokio::time::Instant;
use crate::components::load_cell::LoadCell;

pub struct Scale {
    cells: [LoadCell; 4],
    cell_coefficients: Vec<Vec<f64>>,
    tare_offset: f64,
}

impl Scale {
    pub fn new(phidget_id: i32) -> Result<Self, Box<dyn Error>> {
        let cells = [
            LoadCell::new(phidget_id, 0)?,
            LoadCell::new(phidget_id, 1)?,
            LoadCell::new(phidget_id, 2)?,
            LoadCell::new(phidget_id, 3)?
        ];
        
        Ok(Self {
            cells,
            // TODO: filler coefficients for now
            cell_coefficients: vec![vec![1.]; 4],
            tare_offset: 0.
        })
    }

    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.cells.iter_mut().for_each(|cell| { 
            cell.connect().expect("Load Cell Attachment Failed"); 
        });
        Ok(())
    }
    
    fn get_readings(&self) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
        // Gets each load cell reading from Phidget
        // and returns them in a matrix.

        self.cells.iter().map(|cell| {
            cell.get_reading().map(|reading| vec![reading])
        }).collect()
    }

    fn get_medians(&self, samples: usize, sample_rate: usize) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
        let mut readings: Vec<Vec<f64>> = vec![vec![]; 4];
        let mut medians = vec![0.; 4];
        let delay = Duration::from_millis(1000/sample_rate as u64);
        let _start_time = Instant::now();
        for _sample in 0..samples {
            for cell in 0..self.cells.len() {
                readings[cell].push(self.cells[cell].get_reading()?);
            }
            sleep(delay);
        }
        for cell in 0..self.cells.len() {
            medians[cell] = Scale::median(&mut readings[cell]);
        }
        
        Ok(vec![medians])
    }

    pub fn live_weigh(&self) -> Result<f64,  Box<dyn Error>> {
        // Gets the instantaneous weight measurement
        // from the scale by taking the sum of each
        // load cell's reading, weighted by its 
        // coefficient.

        let readings = self.get_readings()?;
        let weight = LinearSystem::dot(&readings, &self.cell_coefficients)? - self.tare_offset;
        Ok(weight)
        
    }


    pub fn weight_by_median(&self, samples: usize, sample_rate: usize) -> Result<f64, Box<dyn Error>> {
        let mut weights = Vec::new();
        let delay = Duration::from_millis(1000/sample_rate as u64);
        let _start_time = time::Instant::now();
        for _sample in 0..samples {
            weights.push(self.live_weigh()?);
            sleep(delay);
        }
        Ok(Scale::median(&mut weights))
    }

    fn median(weights: &mut Vec<f64>) -> f64 {
        weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = weights.len() / 2;
        weights[middle]

    }

    pub fn tare(&mut self, samples: usize, sample_rate: usize) -> Result<(), Box<dyn Error>> {
        let resting_weight = self.weight_by_median(samples, sample_rate)?;
        self.tare_offset += resting_weight;
        Ok(())
    }


    pub fn calibrate(&mut self, test_mass: f64, samples: usize, sample_rate: usize) -> Result<(), Box<dyn Error>> {
        let mut trial_readings = vec![vec![0.; self.cells.len()]; self.cells.len()];
        let test_mass_vector = vec![vec![test_mass]; self.cells.len()];
        for trial in 0..self.cells.len() {
            println!("Place/move test mass and press key");
            let mut input = String::new();
            let _user_input = io::stdin().read_line(&mut input);
            println!("Weighing...");
            let readings = self.get_medians(samples, sample_rate)?;
            trial_readings[trial].clone_from(&LinearSystem::transpose(&readings)[0]);
        }
        println!("DEBUG: {:?}, {:?}", trial_readings, test_mass_vector);
        let mut system = LinearSystem::new(trial_readings, test_mass_vector)?;
        system.display();
        self.change_coefficients(system.solve()?);
        
        Ok(())
    }

    pub fn change_coefficients(&mut self, coefficients: Vec<Vec<f64>>) {
        self.cell_coefficients = coefficients;
    }

    pub fn diagnose(&self, duration: Duration, sample_rate: usize) -> Result<(Vec<Duration>, Vec<f64>), Box<dyn Error>> {
        let mut times = Vec::new();
        let mut weights = Vec::new();
        let data_interval = Duration::from_secs_f64(1. / (sample_rate as f64));

        let init_time = Instant::now();
        while Instant::now() - init_time < duration {
            weights.push(self.live_weigh()?);
            times.push(Instant::now()-init_time);
            sleep(data_interval);
        }

        Ok((times, weights))
    }

}


#[derive(Debug)]
pub enum ScaleError {
    LoadCellError,
    MatrixError(MatrixError),
    IoError(io::Error),
}


#[test]
fn connect_scale_cells() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    Ok(())
}

#[test]
fn read_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    let readings = scale.get_readings()?;
    println!("Scale Readings: {:?}", readings);
    Ok(())
}

#[test]
fn live_weigh_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    scale.cell_coefficients = vec![vec![-4832237.786999262],
                                   vec![-2679438.3255438516],
                                   vec![-4443388.581829642],
                                   vec![-4666590.62744391],
    ];
    let weight = scale.live_weigh()?;
    println!("Weight: {:?}", weight);

    Ok(())
}

#[test]
fn weigh_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    scale.cell_coefficients = vec![vec![-4832237.786999262],
                                   vec![-2679438.3255438516],
                                   vec![-4443388.581829642],
                                   vec![-4666590.62744391],
    ];
    let weight = scale.weight_by_median(300, 50)?;
    println!("Weight: {:?}", weight);
    
    Ok(())
}

#[test]
fn calibrate_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    scale.calibrate(437.7, 1000, 100)?;

    Ok(())
}

#[test]
fn get_medians() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    let medians = scale.get_medians(1000, 50)?;
    println!("Medians: {:?}", medians);
    Ok(())
}

#[test]
fn diagnose_scale() -> Result<(), Box<dyn Error>> {
    let mut scale = Scale::new(716709)?;
    scale.connect()?;
    let (_times, _weights) = scale.diagnose(Duration::from_secs(5), 100)?;
    Ok(())
}

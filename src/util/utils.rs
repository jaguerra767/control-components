pub struct LowPassFilter {
    a: f64,
    b: f64,
    previous_weight: f64,
}

impl LowPassFilter {
    pub fn new(sample_rate: f64, cutoff_frequency: f64, initial_weight: f64) -> Self {
        let period = 1.0 / sample_rate;
        let rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        Self {
            a: period / (period + rc),
            b: rc / (period + rc),
            previous_weight: initial_weight,
        }
    }

    pub fn apply(&mut self, value: f64) -> f64 {
        self.previous_weight = self.a * value + self.b * self.previous_weight;
        self.previous_weight
    }
}
pub const fn make_prefix(device_type: u8, device_id: u8) -> [u8; 3] {
    [2, device_type, device_id + 48]
}

pub fn num_to_bytes<T: ToString>(number: T) -> Vec<u8> {
    number.to_string().chars().map(|c| c as u8).collect()
}

pub fn int_to_byte(number: u8) -> u8 {
    number + 48
}

pub fn ascii_to_int(bytes: &[u8]) -> isize {
    let sign = if bytes[0] == 45 { -1 } else { 1 };
    let int = bytes
        .iter()
        .filter(|&&x| (48..=57).contains(&x))
        .fold(0, |mut acc, x| {
            let num = x - 48;
            acc *= 10;
            acc += num as isize;
            acc
        });
    int * sign
}

pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(a, b)| a * b).sum::<f64>()
}

pub fn median(weights: &mut [f64]) -> f64 {
    weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let middle = weights.len() / 2;
    weights[middle]
}
#[cfg(test)]
#[test]
fn test_make_prefix() {
    let prefix = make_prefix(77, 2);
    assert_eq!(prefix, [2, 77, 50]);
}

#[test]
fn test_int_to_bytes() {
    let bytes = num_to_bytes(2300);
    assert_eq!(bytes, [50, 51, 48, 48]);
    let bytes = num_to_bytes(-3400);
    assert_eq!(bytes, [45, 51, 52, 48, 48]);
    let bytes = num_to_bytes(2300.0);
    assert_eq!(bytes, [50, 51, 48, 48]);
    let bytes = num_to_bytes(-3400.0);
    assert_eq!(bytes, [45, 51, 52, 48, 48]);
    let bytes = num_to_bytes((-0.5 * 800.0) as isize);
    println!("{:?}", bytes);
}

#[test]
fn test_bytes_to_int() {
    let int = ascii_to_int([45, 51, 52, 48, 48, 13].as_slice());
    assert_eq!(-3400, int);
    let int = ascii_to_int([50, 51, 48, 48].as_slice());
    assert_eq!(2300, int);
}

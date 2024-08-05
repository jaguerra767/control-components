use crate::components::clear_core_io::DigitalOutput;

#[allow(dead_code)]
pub enum Color {
    Red,
    Green,
    Yellow,
}

#[allow(dead_code)]
pub struct Led {
    red_output: DigitalOutput,
    green_output: DigitalOutput,
}

#[allow(dead_code)]
impl Led {
    pub fn new(red_output: DigitalOutput, green_output: DigitalOutput) -> Self {
        Self {
            red_output,
            green_output,
        }
    }

    pub async fn turn_on(&self, color: Color) {
        match color {
            Color::Red => {
                self.green_output.set_state(false).await;
                self.red_output.set_state(true).await;
            }
            Color::Green => {
                self.red_output.set_state(false).await;
                self.green_output.set_state(true).await
            }
            Color::Yellow => {
                self.green_output.set_state(true).await;
                self.red_output.set_state(true).await;
            }
        }
    }

    pub async fn all_off(&self) {
        self.red_output.set_state(false).await;
        self.green_output.set_state(false).await;
    }
}

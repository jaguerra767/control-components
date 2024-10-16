use std::time::Duration;
use crate::components::clear_core_motor::ClearCoreMotor;
use crate::controllers::clear_core::Error;

pub async fn timed_dispense(motor: ClearCoreMotor, motor_speed: f64, duration :Duration) -> Result<(), Error>{
    motor.set_velocity(motor_speed).await?;
    motor.relative_move(1000.).await?;
    tokio::time::sleep(duration).await;
    motor.abrupt_stop().await?;
    Ok(())
}
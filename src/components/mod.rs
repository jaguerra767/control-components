use std::future::Future;

pub mod clear_core_io;
pub mod clear_core_motor;
pub mod load_cell;
pub mod scale;
pub mod send_recv;

pub trait Output {
    fn set_state(&self, state: bool) -> impl Future<Output = ()>;
}

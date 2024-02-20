mod button;
pub use button::*;

mod dot_badge;
pub use dot_badge::*;

#[derive(Clone)]
pub enum ColorOption {
    Blue,
    Green,
    Red,
    Yellow,
    Gray,
}

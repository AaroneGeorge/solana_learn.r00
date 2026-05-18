pub mod initialize;
pub mod deposit;
pub mod withdraw;
pub mod close;

pub use initialize::*;
pub use deposit::*;
pub use withdraw::*;
pub use close::*;

// mod.rs acts as an index.js for the instructions module, 
// re-exporting all the instruction handlers for easy access from lib.rs.
//! # Krossbar state machine
//!
//! Flat state machine used in several Krossbar services.
//!
//! The library provides a structure, which is able to support internal state inside async functions.
//! This allows using it as a client state machine.
//! As a standalone library is not so useful if you don't need its specific functionality.
//!
//! # Examples
//! ```rust
//! use krossbar_state_machine::machine::Machine;
//!
//! async fn up_to_45(mut value: i32) -> Result<i32, i32> {
//!     while value < 45 {
//!         value += 1
//!     }
//!
//!     Ok(value)
//! }
//!
//! fn hello(value: Result<i32, i32>) -> String {
//!     format!("Hello {}!", value.unwrap())
//! }
//!
//! async fn example() {
//!     let mach = Machine::init(42).then(up_to_45).unwrap(hello);
//!
//!     assert_eq!("Hello 45!", mach.await);
//! }
//! ```
pub mod machine;
pub mod state;

pub use machine::Machine;

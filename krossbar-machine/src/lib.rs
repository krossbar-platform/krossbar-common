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
//! use krossbar_machine::{control::Control, machine::Machine};
//!
//! async fn up_to_45(value: i32) -> Control<i32, i32> {
//!     if value < 45 {
//!         Control::Loop(value + 1)
//!     } else {
//!         Control::Return(value)
//!     }
//! }
//!
//! fn hello(value: i32) -> String {
//!     format!("Hello {value}!")
//! }
//!
//! async fn example() {
//!     let mach = Machine::init(42).then(up_to_45).ret(hello);
//!
//!     assert_eq!("Hello 45!", mach.await);
//! }
//! ```
pub mod control;
pub mod machine;
pub mod state;

pub use machine::Machine;

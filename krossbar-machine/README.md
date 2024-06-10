[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/krossbar-machine.svg
[crates-url]: https://crates.io/crates/krossbar-machine
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/krossbar-platform/krossbar-common/blob/main/LICENSE
[actions-badge]: https://github.com/krossbar-platform/krossbar-common/actions/workflows/ci.yml/badge.svg
[actions-url]: https://github.com/krossbar-platform/krossbar-common/actions/workflows/ci.yml

# krossbar-machine

## Krossbar state machine

Flat state machine used in several Krossbar services.

The library provides a structure, which is able to support internal state inside async functions.
This allows using it as a client state machine.
As a standalone library is not so useful if you don't need its specific functionality.

## Examples
```rust
use krossbar_machine::{control::Control, machine::Machine};

async fn up_to_45(value: i32) -> Control<i32, i32> {
    if value < 45 {
        Control::Loop(value + 1)
    } else {
        Control::Return(value)
    }
}

fn hello(value: i32) -> String {
    format!("Hello {value}!")
}

async fn example() {
    let mach = Machine::init(42).then(up_to_45).ret(hello);

    assert_eq!("Hello 45!", mach.await);
}
```

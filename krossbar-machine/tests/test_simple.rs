use krossbar_machine::{control::Control, machine::Machine};

#[tokio::test]
async fn test_init() {
    let mach = Machine::init(42);

    assert_eq!(42, mach.await);
}

async fn inc(value: i32) -> Control<i32, i32> {
    Control::Return(value + 1)
}

#[tokio::test]
async fn test_single_step() {
    let mach = Machine::init(42).then(inc);

    assert_eq!(43, mach.await);
}

#[tokio::test]
async fn test_multiple_step() {
    let mach = Machine::init(42).then(inc).then(inc);

    assert_eq!(44, mach.await);
}

async fn stringify(value: i32) -> Control<i32, String> {
    Control::Return(format!("{value}"))
}

#[tokio::test]
async fn test_change_state_type() {
    let mach = Machine::init(42).then(inc).then(stringify);

    assert_eq!("43", mach.await);
}

async fn up_to_45(value: i32) -> Control<i32, i32> {
    if value < 45 {
        Control::Loop(value + 1)
    } else {
        Control::Return(value)
    }
}

#[tokio::test]
async fn test_simple_loop() {
    let mach = Machine::init(42).then(up_to_45);

    assert_eq!(45, mach.await);
}

fn mul_10(value: i32) -> i32 {
    value * 10
}

#[tokio::test]
async fn test_return() {
    let mach = Machine::init(42).then(up_to_45).ret(mul_10);

    assert_eq!(450, mach.await);
}

fn hello(value: i32) -> String {
    format!("Hello {value}!")
}

#[tokio::test]
async fn all_in_one() {
    let mach = Machine::init(42).then(up_to_45).ret(hello);

    assert_eq!("Hello 45!", mach.await);
}

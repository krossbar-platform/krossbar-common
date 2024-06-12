use krossbar_state_machine::machine::Machine;

#[tokio::test]
async fn test_init() {
    let mach = Machine::<i32, i32>::init(42);

    assert_eq!(Ok(42), mach.await);
}

async fn inc(value: i32) -> Result<i32, i32> {
    Ok(value + 1)
}

#[tokio::test]
async fn test_single_step() {
    let mach = Machine::init(42).then(inc);

    assert_eq!(Ok(43), mach.await);
}

#[tokio::test]
async fn test_multiple_step() {
    let mach = Machine::init(42).then(inc).then(inc);

    assert_eq!(Ok(44), mach.await);
}

async fn stringify(value: i32) -> Result<String, i32> {
    Ok(format!("{value}"))
}

#[tokio::test]
async fn test_change_state_type() {
    let mach = Machine::init(42).then(inc).then(stringify);

    assert_eq!(Ok("43".to_owned()), mach.await);
}

fn mul_10(value: Result<i32, i32>) -> i32 {
    value.unwrap() * 10
}

#[tokio::test]
async fn test_return() {
    let mach = Machine::init(42).then(inc).unwrap(mul_10);

    assert_eq!(430, mach.await);
}

fn hello(value: Result<i32, i32>) -> String {
    format!("Hello {}!", value.unwrap())
}

async fn up_to_45(mut value: i32) -> Result<i32, i32> {
    while value < 45 {
        value += 1
    }

    Ok(value)
}

#[tokio::test]
async fn all_in_one() {
    let mach = Machine::init(42).then(up_to_45).unwrap(hello);

    assert_eq!("Hello 45!", mach.await);
}

async fn make_error(value: i32) -> Result<i32, i32> {
    Err(value)
}

#[tokio::test]
async fn test_early_return() {
    let mach = Machine::init(42).then(make_error).then(inc).then(inc);

    assert_eq!(Err(42), mach.await);
}

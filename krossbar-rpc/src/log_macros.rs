#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => (println!("Error: {}", format!($($arg)+)))
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => (println!("Warning: {}", format!($($arg)+)))
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => (println!("Info: {}", format!($($arg)+)))
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => (println!("Debug: {}", format!($($arg)+)))
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => (println!("Trace: {}", format!($($arg)+)))
}

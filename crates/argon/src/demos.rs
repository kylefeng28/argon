//! Demo programs embedded in the binary and copied to DEMOS/ at startup.

macro_rules! demo {
    ($name:literal) => {
        ($name, include_str!(concat!("../../../disk/DEMOS/", $name)))
    };
}

pub fn all() -> Vec<(&'static str, &'static str)> {
    vec![
        demo!("README.lua"),
        demo!("hello.lua"),
        demo!("snake.lua"),
        demo!("guess.lua"),
    ]
}

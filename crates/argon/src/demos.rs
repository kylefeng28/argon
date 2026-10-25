//! Demo programs embedded in the binary and copied to DEMOS/ at startup.

macro_rules! demo {
    ($name:literal) => {
        ($name, include_str!(concat!("../../../disk/DEMOS/", $name)))
    };
}

pub fn all() -> Vec<(&'static str, &'static str)> {
    vec![
        demo!("README.lua"),
        demo!("beeps.lua"),
        demo!("bounce.lua"),
        demo!("chiptune.lua"),
        demo!("clock.lua"),
        demo!("cube.lua"),
        demo!("guess.lua"),
        demo!("hello.lua"),
        demo!("matrix.lua"),
        demo!("particles.lua"),
        demo!("plot.lua"),
        demo!("pong.lua"),
        demo!("quadratic.lua"),
        demo!("retro.lua"),
        demo!("shapes.lua"),
        demo!("snake.lua"),
        demo!("sprites.lua"),
        demo!("starfield.lua"),
    ]
}

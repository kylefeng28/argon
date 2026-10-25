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

        demo!("cube.lua"),
        demo!("lightcycles.lua"),
        demo!("mandelbrot.lua"),
        demo!("julia.lua"),
        demo!("hud.lua"),
        demo!("life.lua"),
        demo!("flock.lua"),
        demo!("attractor.lua"),
        demo!("mandelbrot_gpu.lua"),
        demo!("julia_gpu.lua"),
        demo!("raymarch.lua"),
        demo!("shaderfx.lua"),
        demo!("particles_gpu.lua"),
        demo!("reaction_diffusion.lua"),
        demo!("shadertoy/README.glsl"),
    ]
}

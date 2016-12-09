use std::f32;
use std::thread::sleep;
use std::time::Duration;

extern crate rand;
use rand::Rng;
use rand::distributions::{IndependentSample,Range};
extern crate sacn;
use sacn::DmxSource;

const DECAY_FACTOR: f32 = 4_f32;
const LIGHT_THRESHOLD: f32 = 0.20;
const MAX_BRIGHTNESS: f32 = 75_f32;
const NUM_LIGHTS: u16 = 20;

struct Pixel { level: i16, age: i16 }
// impl Default for Pixel {
//     fn default() -> Pixel {
//         Pixel { level: 0, age: 0 };
//     }
// }

fn main() {
    let mut rng = rand::thread_rng();
    let mut lights: Vec<Pixel> = vec![];
    for i in 1..NUM_LIGHTS {
        let pixel = Pixel { level: 0, age: 0 };
        lights.push(pixel);
    }

    // half of a second
    let refresh = Duration::new(0, 500_000_000);
    let bright_range = Range::new(0_f32, MAX_BRIGHTNESS);
    let zero_to_one = Range::new(0_f32, 1_f32);
    loop {
        for light in lights.iter_mut() {
            if light.level == 0 {
                // light is currently dark
                // test to see if we want to light it
                if zero_to_one.ind_sample(&mut rng) < LIGHT_THRESHOLD {
                    // we do, so pick a random 
                    light.level = bright_range.ind_sample(&mut rng) as i16;
                    light.age = 1;
                }
            } else {
                // light is lit
                // test to see if we rise or fall
                // probability of falling should go up as light ages
                if zero_to_one.ind_sample(&mut rng) > 1.0/(light.age as f32) {
                    // falling
                    light.level -= (bright_range.ind_sample(&mut rng)/DECAY_FACTOR) as i16;
                    light.age += 1;
                    // test to see if we bottomed out
                    if light.level <= 0 {
                        light.level = 0;
                        light.age = 0;
                    }
                } else {
                    // rising
                    light.level += (zero_to_one.ind_sample(&mut rng) * (MAX_BRIGHTNESS - light.level as f32)) as i16;
                    light.age += 1;
                }
            }
        }
        render(&lights);
        sleep(refresh);
    }
}

fn render( lights: &[Pixel] ) {
    let mut out: Vec<i16> = vec![];
    for light in lights {
        out.push(light.level);
    }
    println!("{:?}", out)
}

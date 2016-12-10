use std::env;
use std::f32;
use std::thread::sleep;
use std::time::Duration;

extern crate getopts;
use getopts::Options;
extern crate rand;
use rand::Rng;
use rand::distributions::{IndependentSample,Range};
extern crate sacn;
use sacn::DmxSource;

const DECAY_FACTOR: f32 = 2_f32;
const LIGHT_THRESHOLD: f32 = 0.10;
const MAX_BRIGHTNESS: f32 = 75_f32;

const UNIVERSE_SIZE: usize = 510;

#[derive(Debug)]
struct Params {
    decay:          f32,
    threshold:      f32,
    max_brightness: f32,
    sleep:          Duration
}

#[derive(Debug)]
#[derive(Clone)]
struct Pixel { level: i16, age: i16 }
// impl Pixel {
//     fn clone(&self) -> Pixel {
//         let mut pixel = Pixel { level: self.level, age: self.age };
//     }
// }

struct Zone  { head: u8, body: u8, tail: u8, name: String }

fn build_params () -> Params {
    // seed default params
    let mut params = Params { decay: 2_f32, threshold: 0.1, max_brightness: 75_f32, sleep: Duration::new(1, 500_000_000) };

    // parse command line args and adjust params accordingly
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("d", "decay", "slow decay by this factor, defaults to 2", "DECAY");
    opts.optopt("t", "threshold", "probablity that a pixel lights up, default 0.10", "THRESHOLD");
    opts.optopt("m", "maxbrightness", "maximum brightness, 1..255, default 75", "MAX");
    opts.optopt("s", "sleep", "sleep interval in seconds, default 1.5", "SECONDS");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("d") {
        params.decay = matches.opt_str("d").unwrap().parse::<f32>().unwrap();
    }
    if matches.opt_present("t") {
        params.threshold = matches.opt_str("t").unwrap().parse::<f32>().unwrap();
    }
    if matches.opt_present("m") {
        params.max_brightness = matches.opt_str("m").unwrap().parse::<f32>().unwrap();
    }
    if matches.opt_present("s") {
        // take float seconds
        // convert to int seconds and nanoseconds to make Duration happy
        let seconds: f32 = matches.opt_str("s").unwrap().parse::<f32>().unwrap();
        let whole_seconds: u64 = seconds as u64;
        let nano_seconds: u32 = ((seconds - whole_seconds as f32) * 1_000_000_000_f32) as u32;
        params.sleep = Duration::new(whole_seconds, nano_seconds);
    }
    return params;
}

fn main() {
    let params = build_params();

    let dmx = DmxSource::new("Controller").unwrap();

    let zones: [Zone; 6] = [
        Zone { head: 0, body: 44, tail: 3, name: "10".to_string() },
        Zone { head: 2, body: 91, tail: 3, name: "11a".to_string() },
        Zone { head: 2, body: 92, tail: 2, name: "11b".to_string() },
        Zone { head: 2, body: 90, tail: 3, name: "12a".to_string() },
        Zone { head: 2, body: 91, tail: 3, name: "12b".to_string() },
        Zone { head: 2, body: 43, tail: 0, name: "13".to_string() }
    ];

    let mut lights: Vec<Pixel> = vec![];
    // TODO: probably a more idiomatic way to built the default state
    for zone in zones.iter() {
        for i in 1..zone.body {
            let pixel = Pixel { level: 0, age: 0 };
            lights.push(pixel);
        }
    }

    let mut rng = rand::thread_rng();
    let bright_range = Range::new(0_f32, params.max_brightness);
    let zero_to_one = Range::new(0_f32, 1_f32);

    loop {
        for light in lights.iter_mut() {
            if light.level == 0 {
                // light is currently dark
                // test to see if we want to light it
                if zero_to_one.ind_sample(&mut rng) < params.threshold {
                    // we do, so pick a random 
                    light.level = bright_range.ind_sample(&mut rng) as i16;
                    light.age += 1;
                }
            } else {
                // light is lit
                // test to see if we rise or fall
                // probability of falling should go up as light ages
                if zero_to_one.ind_sample(&mut rng) > 1.0/(light.age as f32) {
                    // falling
                    light.level -= (bright_range.ind_sample(&mut rng)/params.decay) as i16;
                    light.age += 1;
                    // test to see if we bottomed out
                    if light.level <= 0 {
                        light.level = 0;
                        light.age = 0;
                    }
                } else {
                    // rising
                    light.level += (zero_to_one.ind_sample(&mut rng) * (params.max_brightness - light.level as f32)) as i16;
                    light.age += 1;
                }
            }
        }
        render(&lights, &zones, &dmx);
        sleep(params.sleep);
    }
    dmx.terminate_stream(1);
}

// debug, rendor output to console
// fn render( lights: &[Pixel], zones: &[Zone], dmx: &DmxSource ) {
//    let mut out: Vec<i16> = vec![];
//    for light in lights {
//        out.push(light.level);
//   }
//    println!("{:?}", out)
//     let mut lit: u16 = 0;
//     let mut level: i16 = 0;
//     for light in lights {
//         if light.level > 0 {
//             lit += 1;
//             level += light.level;
//         }
//     }
//     println!("{} pixels, {} lit, {} avg level", lights.len(), lit, (level as f32)/(lit as f32));
// }

// output to lighting controller via sACN
fn render( lights: &[Pixel], zones: &[Zone], dmx: &DmxSource ) {
    let mut out: Vec<u8> = vec![];
    let mut copy: Vec<Pixel> = vec![];
    copy.extend_from_slice(lights);
    let mut idx: usize = 0;
    for zone in zones {
        // null pixels at head
        for i in 1..zone.head {
            out.push(0); // Red
            out.push(0); // Green
            out.push(0); // Blue
        }
        // set via light.level in the body
        for i in 0..(zone.body - 1) {
            out.push(copy[idx].level as u8); // Red
            out.push(copy[idx].level as u8); // Green
            out.push(copy[idx].level as u8); // Blue
            idx += 1;
        }
        // null pixels at tail
        for i in 1..zone.tail {
            out.push(0); // Red
            out.push(0); // Green
            out.push(0); // Blue
        }
    }
    let mut universes = Vec::new();
    while out.len() > UNIVERSE_SIZE {
        let u = out.split_off(UNIVERSE_SIZE);
        universes.push(out);
        out = u;
    }
    universes.push(out);
    let mut universe: u16 = 1;
    for u in universes {
        dmx.send(universe, &u);
        universe += 1;
    }
}


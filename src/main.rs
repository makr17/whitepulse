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

const UNIVERSE_SIZE: usize = 510;

const GAMMA: f32 = 2.2;

#[derive(Debug)]
struct Params {
    decay:         f32,
    threshold:     f32,
    max_intensity: f32,
    sleep:         Duration
}

#[derive(Debug)]
#[derive(Clone)]
struct RGB {
    red:   u8,
    green: u8,
    blue:  u8
}

#[derive(Debug)]
#[derive(Clone)]
struct Pixel { intensity: f32, age: u32, temp: u16, rgb: RGB }

struct Zone  { head: u8, body: u8, tail: u8, name: String }

fn build_params () -> Params {
    // seed default params
    let mut params = Params { decay: 0.002, threshold: 0.001, max_intensity: 0.8_f32, sleep: Duration::new(0, 20_000_000) };

    // parse command line args and adjust params accordingly
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("d", "decay", "slow decay by this factor, defaults to 2", "DECAY");
    opts.optopt("t", "threshold", "probablity that a pixel lights up, default 0.10", "THRESHOLD");
    opts.optopt("m", "maxintensity", "maximum brightness, 1..255, default 75", "MAX");
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
        let max: u8 = matches.opt_str("m").unwrap().parse::<u8>().unwrap();
        params.max_intensity = (max as f32)/255_f32
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

fn kelvin (mut temp: u16) -> RGB {
    // http://www.tannerhelland.com/4435/convert-temperature-rgb-algorithm-code/
    temp /= 100;

    let mut rgb: RGB = RGB { red: 0, green: 0, blue: 0 };
    // calculate red
    if temp <= 66 {
        rgb.red = 255;
    } else {
        let red: f32 = (temp - 60) as f32;
        rgb.red = (329.698727446 * red.powf(-0.1332047592)).round() as u8;
    }
    // calculate green
    if temp <= 66 {
        let green: f32 = temp as f32;
        rgb.green = (99.4708025861 * green.ln() - 161.1195681661).round() as u8;
    } else {
        let green: f32 = (temp - 60) as f32;
        rgb.green = (288.1221695283 * green.powf(-0.0755148492)).round() as u8;
    }
    // calculate blue
    if temp >= 66 {
        rgb.blue = 255;
    } else {
        if temp <= 19 {
            rgb.blue = 0;
        } else {
            let blue: f32 = (temp - 10) as f32;
            rgb.blue = (138.5177312231 * blue.ln() - 305.0447927307).round() as u8;
        }
    }
    return rgb;
}

fn gamma_correct(rgb: &RGB) -> RGB {
    let mut c: RGB = RGB {red: 0, green: 0, blue: 0 };
    c.red   = (255_f32 * (rgb.red   as f32 / 255_f32).powf(GAMMA)) as u8;
    c.green = (255_f32 * (rgb.green as f32 / 255_f32).powf(GAMMA)) as u8;
    c.blue  = (255_f32 * (rgb.blue  as f32 / 255_f32).powf(GAMMA)) as u8;
    return c;
}

fn scale_rgb(rgb: RGB, intensity: f32, params: &Params) -> RGB {
    let i: f32 = intensity * params.max_intensity;
    let scaled: RGB = RGB {
        red:   (rgb.red   as f32 * i).round() as u8,
        green: (rgb.green as f32 * i).round() as u8,
        blue:  (rgb.blue  as f32 * i).round() as u8
    };
    return scaled;
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
            let pixel = Pixel {
                intensity: 0_f32,
                age: 0,
                temp: 0,
                rgb: RGB { red: 0, green: 0, blue: 0 },
            };
            lights.push(pixel);
        }
    }

    let mut rng = rand::thread_rng();
    let zero_to_one = Range::new(0_f32, 1_f32);
    let temp_range = Range::new(2700_f32, 5500_f32);

    loop {
        for light in lights.iter_mut() {
            if light.intensity == 0_f32 {
                // light is currently dark
                // test to see if we want to light it
                if zero_to_one.ind_sample(&mut rng) < params.threshold {
                    // we do, so pick a random 
                    light.intensity = zero_to_one.ind_sample(&mut rng) as f32;
                    light.age += 1;
                    light.temp = temp_range.ind_sample(&mut rng).round() as u16;
                    light.rgb = scale_rgb(kelvin(light.temp), light.intensity, &params);
                }
            } else {
                // light is lit
                // test to see if we rise or fall
                // probability of falling should go up as light ages
                if zero_to_one.ind_sample(&mut rng) > 1.0/(light.age as f32) {
                    // falling
                    light.intensity -= (zero_to_one.ind_sample(&mut rng) * params.decay) as f32;
                    light.age += 1;
                    // test to see if we bottomed out
                    // check if intensity fell below zero
                    if light.intensity <= 0_f32 {
                        light.intensity = 0_f32;
                        light.age = 0;
                        light.temp = 0;
                        light.rgb.red = 0;
                        light.rgb.green = 0;
                        light.rgb.blue = 0;
                    }
                    // count zeroes, to avoid single-color output
                    //   when gamma-correct bottoms out, usually with red
                    let mut zeroes: u8 = 0;
                    if light.rgb.red == 0 { zeroes += 1 };
                    if light.rgb.green == 0 { zeroes += 1 };
                    if light.rgb.blue == 0 { zeroes += 1 };
                    if zeroes >= 2 {
                        light.intensity = 0_f32;
                        light.age = 0;
                        light.temp = 0;
                        light.rgb.red = 0;
                        light.rgb.green = 0;
                        light.rgb.blue = 0;
                    }
                } else {
                    // rising
                    light.intensity += zero_to_one.ind_sample(&mut rng) * params.decay * (params.max_intensity - light.intensity);
                    
                    light.age += 1;
                    light.rgb = scale_rgb(kelvin(light.temp), light.intensity, &params);
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
        for i in 0..zone.head {
            out.push(0); // Red
            out.push(0); // Green
            out.push(0); // Blue
        }
        // set via light.level in the body
        for i in 0..zone.body {
            let ref rgb = copy[idx].rgb;
            let gc = gamma_correct(&rgb);
            out.push(gc.red);
            out.push(gc.green);
            out.push(gc.blue);
            idx += 1;
            // HACK: patching up an off-by-one somewhere
            if idx == copy.len() { break };
        }
        // null pixels at tail
        for i in 0..zone.tail {
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


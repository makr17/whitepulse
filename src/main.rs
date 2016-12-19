use std::env;
use std::f32;
use std::i64;
use std::thread::sleep;

extern crate getopts;
use getopts::Options;
extern crate houselights;
use houselights::houselights::{RGB,Zone,Dmx,kelvin,scale_rgb,gamma_correct,render};
extern crate rand;
use rand::distributions::{IndependentSample,Range};
extern crate time;
use time::Duration;

#[derive(Debug)]
struct Params {
    decay:         f32,
    max_intensity: f32,
    runfor:        i64,
    sleep:         std::time::Duration,
    threshold:     f32
}

#[derive(Debug)]
#[derive(Clone)]
struct Pixel { intensity: f32, age: u32, temp: u16, rgb: RGB }

fn build_params () -> Params {
    // seed default params
    let mut params = Params {
        decay: 0.002,
        max_intensity: 0.8,
        runfor: 5,
        sleep: Duration::nanoseconds(20_000_000).to_std().unwrap(),
        threshold: 0.001
    };

    // parse command line args and adjust params accordingly
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt("d", "decay", "slow decay by this factor, defaults to 2", "DECAY");
    opts.optopt("m", "maxintensity", "maximum brightness, 1..255, default 75", "MAX");
    opts.optopt("r", "runfor", "number of minutes to run, default 5", "MINUTES");
    opts.optopt("s", "sleep", "sleep interval in seconds, default 1.5", "SECONDS");
    opts.optopt("t", "threshold", "probablity that a pixel lights up, default 0.10", "THRESHOLD");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("d") {
        params.decay = matches.opt_str("d").unwrap().parse::<f32>().unwrap();
    }
    if matches.opt_present("m") {
        let max: u8 = matches.opt_str("m").unwrap().parse::<u8>().unwrap();
        params.max_intensity = (max as f32)/255_f32
    }
    if matches.opt_present("r") {
        params.runfor = matches.opt_str("r").unwrap().parse::<i64>().unwrap();
    }
    if matches.opt_present("s") {
        // take float seconds
        // convert to int seconds and nanoseconds to make Duration happy
        let seconds: f32 = matches.opt_str("s").unwrap().parse::<f32>().unwrap();
        let whole_seconds: i64 = seconds as i64;
        let nano_seconds: i64 = ((seconds - whole_seconds as f32) * 1_000_000_000_f32) as i64;
        params.sleep = (Duration::seconds(whole_seconds) + Duration::nanoseconds(nano_seconds)).to_std().unwrap();
    }
    if matches.opt_present("t") {
        params.threshold = matches.opt_str("t").unwrap().parse::<f32>().unwrap();
    }
    return params;
}

fn main() {
    let params = build_params();

    let dmx = Dmx::new();
    
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
        for _i in 1..zone.body {
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

    let finish = time::get_time() + Duration::minutes(params.runfor);
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
                    light.rgb = scale_rgb(kelvin(light.temp), light.intensity, params.max_intensity);
                }
            } else {
                // light is lit
                // test to see if we rise or fall
                // probability of falling should go up as light ages
                if zero_to_one.ind_sample(&mut rng) > 1.0/(light.age as f32) {
                    // falling
                    light.intensity -= (zero_to_one.ind_sample(&mut rng) * params.decay) as f32;

                    light.age += 1;
                    // TODO: a test floor that doesn't involve redundant gamma calculations
                    let gamma: RGB = gamma_correct(&light.rgb);
                    if (gamma.red as u16 + gamma.green as u16 + gamma.blue as u16) < 20 {
                        light.intensity = 0_f32;
                        light.rgb = RGB::null();
                        light.age = 0;
                    }
                } else {
                    // rising
                    light.intensity += zero_to_one.ind_sample(&mut rng) * params.decay * (params.max_intensity - light.intensity);
                    
                    light.age += 1;
                    light.rgb = scale_rgb(kelvin(light.temp), light.intensity, params.max_intensity);
                }
            }
        }
        // extract rgb structs from vector of pixels
        let rgb: Vec<RGB> = lights.clone().into_iter().map(|x| x.rgb).collect();
        // and send it as a slice to render()
        render(&rgb, &zones, &dmx);
        if time::get_time() > finish {
            break;
        }
        sleep(params.sleep);
    }
}

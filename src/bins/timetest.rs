use std::time::{Duration, Instant};
use tokio::time;
use clap::Parser;

/// Simple program that sends it's duration at the given frequency period
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Delay between sending the duration since program start, in seconds
    #[arg(short, long)]
    delay: f64,
}

fn temperature(time: f64) -> f64 {
    const AMPLITUDE: f64 = 5.0; // Celsius
    const FREQUENCY: f64 = 1.0; // Hz
    const AVERAGE: f64 = 12.0; // Celsius
    AMPLITUDE * (FREQUENCY * 2.0 * std::f64::consts::PI * time).sin() + AVERAGE
}

fn humidity(time: f64) -> f64 {
    const AMPLITUDE: f64 = 5.0; // RH
    const FREQUENCY: f64 = 0.1; // Hz
    const AVERAGE: f64 = 27.0; // RH
    AMPLITUDE * (FREQUENCY * 2.0 * std::f64::consts::PI * time).sin() + AVERAGE
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let beginning = Instant::now();
    let mut interval = time::interval(Duration::from_secs_f64(args.delay));
    interval.tick().await;
    loop {
        let now = Instant::now();
        let time = now.duration_since(beginning).as_secs_f64();
        let temperature = temperature(time);
        let humidity = humidity(time);
        println!("{},{},{}", time, temperature, humidity);
        interval.tick().await;
    }
}

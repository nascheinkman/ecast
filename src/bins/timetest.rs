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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let beginning = Instant::now();
    let mut interval = time::interval(Duration::from_secs_f64(args.delay));
    interval.tick().await;
    loop {
        let now = Instant::now();
        println!("{}", now.duration_since(beginning).as_secs_f64());
        interval.tick().await;
    }
}

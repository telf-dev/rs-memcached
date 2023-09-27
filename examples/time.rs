use std::time::{Duration, Instant, SystemTime};
use std::thread::{sleep};

fn main(){
    let now = Instant::now();
    let sysNow = SystemTime::now();

    println!("now: {:?}, sysNow: {:?}", now, sysNow);

}
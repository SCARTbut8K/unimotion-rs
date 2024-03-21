#![allow(unused_must_use)]

use std::time::Duration;

// use unimotion_rs::unimotion::lights::*;
// use unimotion_rs::unimotion::{UnimotionManager, SimpleUnimotionDriver};
use unimotion_rs::prelude::*;

fn main() -> UnimotionResult<()> {
    let manager = UnimotionManager::get_instance();
    
    // std::thread::sleep(Duration::from_secs(5));

    let mut manager = match manager.lock() {
        Ok(m) => m,
        Err(m) => m.into_inner(),
    };
    // manager.send_command(Command::RequestSensorInfo(255));
    manager.send_command(Command::ListSensor);

    loop {
        std::thread::sleep(Duration::from_secs(1));   
    }
    Ok(())
}

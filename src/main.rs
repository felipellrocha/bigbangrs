mod app;
mod texture;

use crate::app::App;

fn main() {
    //env_logger::init();
    log::info!("Starting...");
    let _ = App::run();
}

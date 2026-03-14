mod app;

use crate::app::App;

fn main() {
    //env_logger::init();
    log::info!("Hello, world!");
    let _ = App::run();
}

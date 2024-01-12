mod cloth_sim_app;
mod sim_gen;

use crate::cloth_sim_app::ClothSimApp;
use wgpu_bootstrap::runner::Runner;

fn main() {
    let mut runner = pollster::block_on(Runner::new());

    let app = ClothSimApp::new(&mut runner.context);

    runner.start(app);
}

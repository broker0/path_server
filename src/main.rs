use std;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::world::{WorldModel, WorldTile};

use mul::*;
use world::world_model::WorldData;
use crate::ui::viewer::run_app;

mod ui;
mod mul;
mod http;
mod world;


// fn test_pathfinding() {
//     let world_model = WorldModel::new(Arc::new(WorldData::new()));
//     let world_model = &world_model;
//     world_model.load_state("utopia.save");
//
//     let pf = WorldSurveyor::new(world_model.world(0));
//
//     let (x, y, z) = (1438, 1696, 0);
//     let mut points = Vec::new();
//
//     // pf.trace_area(x, y, z, 3433, 111, 0, &mut points, &TraceOptions::empty());
//     // pf.a_star_trace(802, 1921, 0, 0,
//     //                 // 1630, 622, 22, 0);
//     //                 // 2024, 836, 0, 6);    // brit desert
//     //                 // 2558, 500, 0, 0);    // minoc cave
//     //                 // 1606, 3547, 0, 0);
//     //                // 1520, 938, 0, 7);
//     //                 3433, 111, 0, 7);   // vesper
// }


fn run_service() {

    let start = Instant::now();
    println!("loading data from files, creating the world...");
    let world_model = Arc::new(WorldModel::new(Arc::new(WorldData::new())));
    // patch_world(world_model.clone());
    println!("the creation completed in {:?}", start.elapsed());

    let (http_stop_tx, http_stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = {
        let model = world_model.clone();
        std::thread::spawn(move || {
            http::http_server_service(model, http_stop_rx);
        })
    };

    // TODO start condition from config/command line
    if true {
        run_app(world_model);
    }

    println!("app stopped");
    http_stop_tx.send(()).unwrap();
    handle.join().unwrap();
    std::thread::sleep(Duration::from_secs(5));
}


fn main() {
    // run_pathfinding();
    // run_app();

    run_service();
}

use std;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::cell::{RefCell};
use std::thread::JoinHandle;
use tokio::sync::oneshot::Sender;

use crate::world::{WorldModel, WorldTile};

mod mul;
mod http;
mod world;

use mul::*;
use world::world_model::WorldData;


struct ServerControl {
    stop_signal: Sender<()>,
    handle: JoinHandle<()>,
}


thread_local! {
    static SERVER_CONTROL: RefCell<Option<ServerControl>> = RefCell::new(None);
}


#[no_mangle]
pub extern "C" fn start_path_server() -> bool {
    println!("start path server");
    SERVER_CONTROL.with(|control| {
        match control.borrow().as_ref() {
            Some(_) => return false,
            None => {}
        }

        control.replace(run_service());

        true
    })
}


#[no_mangle]
pub extern "C" fn stop_path_server() -> bool {
    println!("stop path server");
    SERVER_CONTROL.with(|control| {
        let control = control.replace(None);
        match control {
            None => return false,
            Some(control) => {
                control.stop_signal.send(()).unwrap();
                control.handle.join().unwrap();
            }
        }

        true
    })
}


fn run_service() -> Option<ServerControl> {
    let world_model = Arc::new(WorldModel::new(Arc::new(WorldData::new())));
    let (http_stop_tx, http_stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = {
        let model = world_model.clone();
        std::thread::spawn(move || {
            http::http_server_service(model, http_stop_rx);
        })
    };

    Some(ServerControl{
        stop_signal: http_stop_tx,
        handle,
    })
}

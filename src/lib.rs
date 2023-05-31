use std;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::cell::{RefCell};
use std::thread::JoinHandle;
use tokio::sync::oneshot::Sender;
use lazy_static::lazy_static;
use std::sync::Mutex;
use log::{debug, info, warn};

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


lazy_static! {
    static ref SERVER_CONTROL: Mutex<Option<ServerControl>> = Mutex::new(None);
}


#[no_mangle]
pub extern "C" fn start_path_server() -> bool {
    info!("try start path server");
    {
        let mut control = SERVER_CONTROL.lock().unwrap();
        if control.is_some() {
            warn!("path_server already started");
            return false;
        }

        *control = run_service();
        debug!("path_server started");
    }
    true
}


#[no_mangle]
pub extern "C" fn stop_path_server() -> bool {
    info!("try stop path server");
    {
        let mut control = SERVER_CONTROL.lock().unwrap();
        match control.take() {
            None => {
                warn!("path_server already stopped");
                return false
            },

            Some(control) => {
                control.stop_signal.send(()).unwrap();
                control.handle.join().unwrap();
                debug!("path_server stopped");
            }
        }
    }
    true
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

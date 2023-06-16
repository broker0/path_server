use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use log::{debug, warn};
use crate::world::DynamicWorld;

use tokio::time::Instant;
use crate::http::server::{Item, MultiItemPart};
use crate::mul::colordata::ColorData;
use crate::mul::{Multi, TileData};
use crate::world::tiles::TopLevelItem;
use crate::world::world::StaticWorld;

use serde::{Deserialize, Serialize};


/// Stores data that is not a map or static
pub struct WorldData {
    pub colors: ColorData,  // data from radarcol.mul
    pub tiledata: TileData, // data from tiledata.mul
    pub multis: Multi,      // data from multi.idx multi.mul
    pub custom_multis: RwLock<HashMap<u32, Vec<MultiItemPart>>>,
    // etc
}

impl WorldData {
    pub fn new(data_path: &Path) -> Self {
        WorldData {
            colors: ColorData::read(data_path).unwrap(),
            tiledata: TileData::read(data_path).unwrap(),
            multis: Multi::read(data_path).unwrap(),
            custom_multis: RwLock::new(HashMap::new()),
        }
    }
}


#[derive(Serialize, Deserialize)]
struct WorldState {
    pub custom_multis: HashMap<u32, Vec<MultiItemPart>>,
    pub items_index: HashMap<u32, TopLevelItem>
}


pub struct WorldModel {
    pub data: Arc<WorldData>,
    worlds: HashMap<u8, DynamicWorld>,

    // TODO replace HashMap with HashSet by hashing TopLevelItem only over the serial field
    pub items_index: RwLock<HashMap<u32, TopLevelItem>>,
}

impl WorldModel {
    pub fn new(data_path: &Path) -> Self {
        let mut result = WorldModel {
            data: Arc::new(WorldData::new(data_path)),
            worlds: HashMap::new(),

            items_index: RwLock::new(HashMap::new()),
        };

        let world_specs = [(0, 768, 512), (1, 768, 512), (2, 288, 200), (3, 320, 256), (4, 181, 181), (5, 160, 512)];
        for (world, w,h) in world_specs {
            match StaticWorld::probe(data_path, world, w, h) {
                Some((w, h)) => {
                    result.worlds.insert(world, DynamicWorld::new(data_path, result.data.clone(), world, w, h));
                    debug!("world {world} is loaded");
                }
                None => {
                    debug!("No files found for world {world}")
                }
            }
        }

        result
    }


    pub fn world(&self, n: u8) -> &DynamicWorld {
        let world = self.worlds.get(&n);
        match world {
            Some(world) => world,
            None => unreachable!(),
        }
    }


    pub fn save_state(&self, file_name: &str) {
        let state = {
            let multis = self.data.custom_multis.read().unwrap();
            let items = self.items_index.read().unwrap();

            let ws = WorldState {
              custom_multis: multis.clone(),
                items_index: items.clone(),
            };

            serde_json::to_string_pretty(&ws).unwrap()
        };

        let mut file = File::create(file_name).unwrap();
        file.write_all(state.as_bytes()).unwrap();
    }


    pub fn load_state(&self, file_name: &str) {
        let mut file = File::open(file_name).unwrap();
        let mut state = String::new();
        file.read_to_string(&mut state).unwrap();   // count of bytes readed

        let ws: WorldState = serde_json::from_str(&state).unwrap();
        self.clear_state();

        {
            let mut multis = self.data.custom_multis.write().unwrap();
            multis.clone_from(&ws.custom_multis);
        }

        for (_, item) in ws.items_index {
            self.insert_item(item);
        }
    }


    pub fn clear_state(&self) {
        let mut items = HashMap::new();
        {   // we must drop ReadGuard before delete items
            let index = self.items_index.read().unwrap();
            for (&key, &TopLevelItem{ world, x, y, z, serial, graphic, timestamp: last_updated }) in index.iter() {
                items.insert(key, TopLevelItem{ world, x, y, z, serial, graphic, timestamp: last_updated });
            }
        }

        for (_, TopLevelItem{ serial, .. }) in &items {
            self.delete_item(*serial);
        }

        let mut multis = self.data.custom_multis.write().unwrap();
        multis.clear();
    }


    pub fn delete_item(&self, serial: u32) {
        let mut index = self.items_index.write().unwrap();
        let item = index.get(&serial);

        if let Some(&TopLevelItem{ world, x, y, z, serial, graphic, .. }) = item {
            let world = self.world(world);
            world.delete_item(x, y, z, serial, graphic);
            index.remove(&serial);
        }

    }


    pub fn insert_item(&self, item: TopLevelItem) {
        let mut index = self.items_index.write().unwrap();

        // delete old item
        let old = index.remove(&item.serial);
        if let Some(TopLevelItem{ world, x, y, z, serial, graphic , .. }) = old {
            let world_model = self.world(world);
            world_model.delete_item(x, y, z, serial, graphic);
        }

        let world_model = self.world(item.world);
        world_model.insert_item(item.x, item.y, item.z, item.serial, item.graphic, );

        // insert new
        index.insert(item.serial, item);

    }


    pub fn insert_multi_item(&self, item: TopLevelItem, parts: &Vec<MultiItemPart>) {
        let mut index = self.items_index.write().unwrap();

        // try delete main item from index
        let old = index.remove(&item.serial);
        if let Some(TopLevelItem{ world, x, y, z, serial, graphic, .. }) = old {
            let world_model = self.world(world);
            world_model.delete_item(x, y, z, serial, graphic);
        }

        // update custom_multis parts
        if let Ok(mut custom_multis) = self.data.custom_multis.write() {
            custom_multis.remove(&item.serial);
            custom_multis.insert(item.serial, parts.clone());
        }

        // insert multi-parts to the world
        let world_model = self.world(item.world);
        world_model.insert_item(item.x, item.y, item.z, item.serial, item.graphic);

        index.insert(item.serial, item);    // insert main item to index
    }


    pub fn query(&self, world: u8, left: isize, top: isize, right: isize, bottom: isize, items: &mut Vec<Item>) {
        let d_world = self.world(world);

        let s = Instant::now();
        d_world.query_area_dynamic(world, left, top, right, bottom, items);
        {
            let index = self.items_index.read().unwrap();
            for item in items.iter_mut() {
                match index.get(&item.serial) {
                    Some(index_item) => item.timestamp = Some(index_item.timestamp),
                    None => warn!("Cannot find item with serial {}", item.serial),
                }
            }
        }

        debug!("({left}, {top})-({right}, {bottom}) found {} items at {:?}", items.len(), s.elapsed());
    }
}

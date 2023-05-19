use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use crate::world::DynamicWorld;

use tokio::time::Instant;
use crate::http::server::Item;
use crate::mul::colordata::ColorData;
use crate::mul::{Multi, TileData};
use crate::world::tiles::TopLevelItem;



/// Stores data that is not a map or static
pub struct WorldData {
    pub colors: ColorData,  // data from radarcol.mul
    pub tiledata: TileData, // data from tiledata.mul
    pub multis: Multi,      // data from multi.idx multi.mul
    // etc
}

impl WorldData {
    pub fn new() -> Self {
        WorldData {
            colors: ColorData::read().unwrap(),
            tiledata: TileData::read().unwrap(),
            multis: Multi::read().unwrap(),
        }
    }
}


pub struct WorldModel {
    pub data: Arc<WorldData>,

    // TODO
    /*
        world 0: 768x512
        world 1: 768x512
        world 2: 288x200
        world 3: 320x256
        world 4: 181x181
        world 5: 160x512
     */
    world0: DynamicWorld,
    world2: DynamicWorld,

    // TODO replace HashMap with HashSet by hashing TopLevelItem only over the serial field
    pub items_index: RwLock<HashMap<u32, TopLevelItem>>,
}

impl WorldModel {
    pub fn new(data: Arc<WorldData>) -> Self {
        WorldModel {
            data: data.clone(),
            world0: DynamicWorld::new(data.clone(), 0, 768, 512),
            world2: DynamicWorld::new(data.clone(),2, 288, 200),

            items_index: RwLock::new(HashMap::new()),
        }
    }


    pub fn world(&self, n: u8) -> &DynamicWorld {
        match n {
            0 => &self.world0,
            2 => &self.world2,
            _ => unreachable!(),
        }
    }


    pub fn save_state(&self, file_name: &str) {
        let state = {
            let r = self.items_index.read().unwrap();
            serde_json::to_string_pretty(&*r).unwrap()
        };

        let mut file = File::create(file_name).unwrap();
        file.write_all(state.as_bytes()).unwrap();
    }


    pub fn load_state(&self, file_name: &str) {
        let mut file = File::open(file_name).unwrap();
        let mut state = String::with_capacity(1024);
        file.read_to_string(&mut state).unwrap();   // count of bytes readed

        self.clear_state();
        let items: HashMap<u32, TopLevelItem> = serde_json::from_str(&state).unwrap();

        for (_, TopLevelItem{ world, x, y, z, serial, graphic, last_updated}) in items {
            self.insert_item(world, x, y, z, serial, graphic, last_updated);
        }
    }


    pub fn clear_state(&self) {
        let mut items = HashMap::new();
        {   // we must drop ReadGuard before delete items
            let index = self.items_index.read().unwrap();
            for (&key, &TopLevelItem{ world, x, y, z, serial, graphic, last_updated }) in index.iter() {
                items.insert(key, TopLevelItem{ world, x, y, z, serial, graphic, last_updated });
            }
        }

        for (_, TopLevelItem{ serial, .. }) in &items {
            self.delete_item(*serial);
        }

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


    pub fn insert_item(&self, world: u8, x: isize, y: isize, z: i8, serial: u32, graphic: u16, last_updated: u64) {
        let mut index = self.items_index.write().unwrap();
        let world_model = self.world(world);

        // delete old item
        let old = index.remove(&serial);
        if let Some(TopLevelItem{ world, x, y, z, serial, graphic , .. }) = old {
            let world_model = self.world(world);
            world_model.delete_item(x, y, z, serial, graphic);
        }

        // insert new
        index.insert(serial, TopLevelItem{world, x, y, z, serial, graphic, last_updated});

        world_model.insert_item(x, y, z, serial, graphic, );
    }


    pub fn query(&self, world: u8, left: isize, top: isize, right: isize, bottom: isize, items: &mut Vec<Item>) {
        let d_world = self.world(world);

        let s = Instant::now();
        d_world.query_area_dynamic(world, left, top, right, bottom, items);
        {
            let index = self.items_index.read().unwrap();
            for item in items.iter_mut() {
                match index.get(&item.serial) {
                    Some(index_item) => item.last_updated = Some(index_item.last_updated),
                    None => println!("Cannot find item with serial {}", item.serial),
                }
            }
        }

        println!("({left}, {top})-({right}, {bottom}) found {} items at {:?}", items.len(), s.elapsed());
    }
}

use std::collections::hash_map::Entry;
use std::fs;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use log::{debug, trace, warn};
use crate::*;
use crate::http::server::Item;
use crate::mapdata::LandBlock;
use crate::staticdata::StaticTile;
use crate::world::tiles::DynamicWorldObject;
use crate::world::{TileShape, TileType};

/// Basic World representation
/// Stores world size information in XxY blocks and also stores information
/// about the map and statics - fields land and statics.
/// Has some functions for working with coordinates
pub struct StaticWorld {
    width_blocks: usize,             // world width in blocks
    height_blocks: usize,            // world height
    pub land: Land,                      // source of land data
    pub statics: Static,                 // source of static data
}

impl StaticWorld {
    pub fn read(data_path: &Path, world: u8, use_mul: bool, width_blocks: usize, height_blocks: usize) -> Self {
        let land = if use_mul {
            Land::read_mul(data_path, world, width_blocks, height_blocks).unwrap()
        } else {
            Land::read_uop(data_path, world, width_blocks, height_blocks).unwrap()
        };

        Self {
            width_blocks,
            height_blocks,
            land,
            statics: Static::read(data_path, world, width_blocks, height_blocks).unwrap(),
        }
    }

    /// checks for the existence of data files and returns the type and dimensions of the world.
    /// true means old format MUL, false means new format UOP.
    pub fn probe(data_path: &Path, world: u8, width: usize, height: usize) -> Option<(bool, usize, usize)> {
        match fs::metadata(data_path.join(format!("map{world}.mul"))) {
            Ok(_) => {
                let width = Land::calc_mul_width(data_path, world, height);
                debug!("found MUL file for world {world} with size {width}x{height} blocks");
                Some((true, width, height))
            },

            Err(_) => {
                match fs::metadata(data_path.join(format!("map{world}LegacyMUL.uop"))) {
                    Ok(_) => {
                        let width = Land::calc_uop_width(data_path, world, height);
                        debug!("found UOP file for world {world} with size {width}x{height} blocks");
                        Some((false, width, height))
                    },
                    Err(_) => None,
                }
            }
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width_blocks*8
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height_blocks*8
    }

    #[inline]
    pub fn normalize_tiles(&self, x: isize, y: isize) -> (isize, isize) {
        (x.rem_euclid(self.width_blocks as isize*8), y.rem_euclid(self.height_blocks as isize*8))
    }

    #[inline]
    pub fn normalize_blocks(&self, bx: isize, by: isize) -> (isize, isize) {
        (bx.rem_euclid(self.width_blocks as isize), by.rem_euclid(self.height_blocks as isize))
    }

    #[inline]
    pub fn block_index(&self, bx: isize, by: isize) -> usize {
        let (bx, by) = self.normalize_blocks(bx, by);

        bx as usize * self.height_blocks + by as usize
    }

    pub fn tile_offsets(&self, x: isize, y: isize) -> (isize, isize) {
        let (x, y) = self.normalize_tiles(x, y);
        (x % 8, y % 8)
    }

    // equivalent of (block(tile_block(x, y)), tile_offsets(x, y))
    #[inline]
    pub fn tile_to_block_offsets(&self, x: isize, y: isize) -> (usize, (usize, usize)) {
        let (x, y) = self.normalize_tiles(x, y);

        let (bx, ox) = (x / 8, x % 8);
        let (by, oy) = (y / 8, y % 8);

        let index = bx as usize * self.height_blocks + by as usize;

        (index, (ox as usize, oy as usize))
    }

    pub fn blocks(&self, index: usize) -> (&LandBlock, &[StaticTile]) {
        (self.land.land_block(index), self.statics.statics_block(index))
    }

    /// returns the z coordinate for the vertex given by x, y coordinates
    #[inline]
    pub fn land_vertex_z(&self, x: isize, y: isize) -> i8 {
        let (index, (ox, oy)) = self.tile_to_block_offsets(x, y);
        let land_block = self.land.land_block(index);
        let land_tile = &land_block[ox][oy];

        land_tile.z
    }

    /// for a land tile with x, y coordinates, returns the minimum,
    /// standing and exit z coordinate for the given direction
    pub fn land_tile_z_stand(&self, x: isize, y: isize, direction: u8) -> (i8, i8, i8) {
        // get the coordinates of all four vertices of the tile
        let left   = self.land_vertex_z(x+0, y+0) as i16;
        let bottom = self.land_vertex_z(x+1, y+0) as i16;
        let right  = self.land_vertex_z(x+1, y+1) as i16;
        let top    = self.land_vertex_z(x+0, y+1) as i16;

        // minimal z of this tile, used as z_base of tile
        let min_z = left.min(right.min(top.min(bottom)));

        // calculate the z-coordinate for standing in this tile,
        // using the pair of vertices with the smallest height difference
        // if height difference is equal then use left-right pair
        let standing_z = if (left - right).abs() > (top - bottom).abs() {
            top + bottom
        } else {
            left + right
        };
        let standing_z = if standing_z < 0 { standing_z-1 } else { standing_z } / 2;

        // calculate the coordinate z when leaving the tile in the specified direction.
        // one or two vertices are used, depending on whether the given direction is straight or diagonal
        let exit_z = match direction & 7 {
            0 => (left + bottom) / 2,  // (0,0)-(1,0)
            1 =>  bottom,              // (1,0)
            2 => (bottom + right) / 2, // (1,0)-(1,1)
            3 =>  right,               // (1,1)
            4 => (right + top) / 2,    // (1,1)-(0,1)
            5 =>  top,                 // (0,1)
            6 => (top + left) / 2,     // (0,1)-(0,0)
            7 =>  left,                // (0,0)
            _ => unreachable!("invalid direction {direction}"),
        };

        (min_z as i8, standing_z as i8, exit_z as i8)
    }
}


type OverlayCache = HashMap<usize, BTreeSet<DynamicWorldObject>>;
type OverlayCacheLock = RwLock<OverlayCache>;
type WriteCache<'a> = RwLockWriteGuard<'a, OverlayCache>;
type ReadCache<'a> = RwLockReadGuard<'a, OverlayCache>;

/// stores information about items in the world that are not static or a map.
/// the data is divided into blocks of 8x8 tiles, just like in the map and statics.
/// each block stores a set of unique elements for a quick search for items with a specific coordinate
pub struct DynamicWorld {
    pub data: Arc<WorldData>,
    pub base: StaticWorld,
    overlay_blocks: OverlayCacheLock,
}



impl DynamicWorld {
    pub fn new(data_path: &Path, world_data: Arc<WorldData>, world: u8, use_mul: bool, width_blocks: usize, height_blocks: usize) -> Self {
        let result = DynamicWorld {
            data: world_data,
            base: StaticWorld::read(data_path, world, use_mul, width_blocks, height_blocks),
            overlay_blocks: RwLock::new(HashMap::new()),
        };

        result
    }

    #[inline]
    pub fn write_overlay(&self) -> WriteCache {
        self.overlay_blocks.write().unwrap()
    }

    #[inline]
    pub fn read_overlay(&self) -> ReadCache {
        self.overlay_blocks.read().unwrap()
    }

    fn overlay_insert_item(&self, overlay: &mut WriteCache, item: DynamicWorldObject) {
        let (x,y) = match item {
            DynamicWorldObject::MultiPart {x, y,  .. } |
            DynamicWorldObject::GameObject {x, y, .. } => (x, y)
        };
        let (block_index, _) = self.base.tile_to_block_offsets(x, y);

        match overlay.entry(block_index) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert(item);
            }

            Entry::Vacant(entry) => {
                let mut block = BTreeSet::new();
                block.insert(item);
                entry.insert(block);
            }
        }
    }

    fn overlay_delete_item(&self, overlay: &mut WriteCache, item: &DynamicWorldObject) -> bool {
        let (&x, &y) = match item {
            DynamicWorldObject::MultiPart {x, y, .. } |
            DynamicWorldObject::GameObject {x, y, .. } => (x, y,)
        };
        let (block_index, _) = self.base.tile_to_block_offsets(x, y);

        match overlay.entry(block_index) {
            Entry::Occupied(mut v) => {
                let removed = v.get_mut().remove(item);
                if v.get().is_empty() {
                    v.remove_entry();
                }
                removed
            },
            Entry::Vacant(_) => false,
        }
    }

    fn overlay_insert_multi_parts(&self, overlay: &mut WriteCache, item: DynamicWorldObject) {
        let (serial, graphic, x, y, z) = match item {
            DynamicWorldObject::GameObject { x, y, z, serial, graphic } => (serial, graphic, x, y, z,),
            _ => unreachable!(),
        };

        let insert = |overlay: &mut WriteCache, x: isize, y: isize, z: i8, tile: u16, parent: u32, counter: u16| {
            self.overlay_insert_item(overlay, DynamicWorldObject::MultiPart { x, y, z,
                tile: tile as u32,
                parent,
                counter,
            })
        };

        assert_ne!(graphic & 0x30000, 0);

        if graphic & 0x10000 != 0 { // standard multi
            let multi = &self.data.multis;
            let multi_id = (graphic & 0xFFFF) as u16;
            let multi_parts = multi.multi_parts(multi_id);

            for (counter, part) in multi_parts.iter().enumerate() {
                let x = x + part.x as isize;
                let y = y + part.y as isize;
                let z = z + part.z as i8;
                insert(overlay, x, y, z, part.static_tile, serial, counter as u16);
            };
        } else if graphic & 0x20000 != 0 {  // custom multi
            let custom_multis = self.data.custom_multis.read().unwrap();
            let multi_parts = custom_multis.get(&serial);

            if let Some(multi_parts) = multi_parts {
                trace!("found parts for multi-object {serial}");

                for (counter, part)  in multi_parts.iter().enumerate() {
                    insert(overlay, part.x, part.y, part.z, part.graphic, serial, counter as u16);
                }
            } else {
                warn!("no parts found for multi-object {serial}")
            }
        }
    }

    fn overlay_delete_multi_parts(&self, overlay: &mut WriteCache, item: &DynamicWorldObject) {
        let (serial, graphic, x, y, z) = match item {
            DynamicWorldObject::GameObject { x, y, z, serial, graphic } => (serial, graphic, x, y, z,),
            _ => unreachable!(),
        };

        assert_ne!(graphic & 0x30000, 0);

        if graphic & 0x10000 != 0 {
            let multi = &self.data.multis;
            let multi_id = (graphic & 0xFFFF) as u16;
            let multi_parts = multi.multi_parts(multi_id);

            for (counter, part) in multi_parts.iter().enumerate() {
                let x = x + part.x as isize;
                let y = y + part.y as isize;
                let z = z + part.z as i8;

                self.overlay_delete_item(overlay, &DynamicWorldObject::MultiPart { x, y, z,
                    tile: part.static_tile as u32,
                    parent: *serial,
                    counter: counter as u16,
                });
            }
        } else if graphic & 0x20000 != 0 {
            let custom_multis = self.data.custom_multis.read().unwrap();
            let multi_parts = custom_multis.get(&serial);
            if let Some(multi_parts) = multi_parts {
                trace!("found parts for multi-object {serial}");

                for (counter, part) in multi_parts.iter().enumerate() {
                    let x = part.x;
                    let y = part.y;
                    let z = part.z;
                    self.overlay_delete_item(overlay, &DynamicWorldObject::MultiPart { x, y, z,
                        tile: part.graphic as u32,
                        parent: *serial,
                        counter: counter as u16,
                    });
                }
            }
        }
    }

    pub fn insert_item(&self, x: isize, y: isize, z: i8, serial: u32, graphic: u32) {
        let item = DynamicWorldObject::game_object(x, y, z, serial, graphic);
        let mut overlay = self.write_overlay();

        if graphic & 0x30000 != 0 {  // multi-object
            self.overlay_insert_multi_parts(&mut overlay, item);  // add parts of multi-object
        }
        self.overlay_insert_item(&mut overlay, item); // add the multi-object itself to the world
    }

    pub fn delete_item(&self, x: isize, y: isize, z: i8, serial: u32, graphic: u32) {
        let item = DynamicWorldObject::game_object(x, y, z, serial, graphic);
        let mut overlay = self.write_overlay();

        if graphic & 0x30000 != 0 {
            self.overlay_delete_multi_parts(&mut overlay, &item);
        }

        self.overlay_delete_item(&mut overlay, &item);
    }

    pub fn clear_world(&self) {
        let mut overlay = self.write_overlay();
        overlay.clear();
    }


    #[inline]
    pub fn world_tile_flag(&self, tile: &WorldTile) -> u32 {
        let tiledata = &self.data.tiledata;
        match tile.tile {
            TileType::MapTile(num) => tiledata.get_land_tile(num).flags,
            TileType::ObjectTile(num) => tiledata.get_static_tile(num).flags,
        }
    }

    #[inline]
    pub fn world_tile_color(&self, tile: &WorldTile) -> (u8, u8, u8, u8) {
        let colors = &self.data.colors;
        match tile.tile {
            TileType::MapTile(num) => colors.get_land_color(num),
            TileType::ObjectTile(num) => colors.get_static_color(num),
        }
    }

    /// returns a `WorldTile` structure for the map tile, given the direction of travel.
    pub fn query_tile_ground(&self, x: isize, y: isize, direction: u8, walkable: u32) -> WorldTile {
        let (idx, (ox, oy)) = self.base.tile_to_block_offsets(x, y);
        let tiledata = &self.data.tiledata;

        let map = self.base.land.land_block(idx);
        let map_tile = map[ox][oy];

        let (z_base, z_stand, z_exit) = self.base.land_tile_z_stand(x, y, direction);
        let z_top = if z_exit > z_stand { z_exit } else { z_stand };

        let tile = TileType::MapTile(map_tile.land_tile);
        let shape = TileShape::from_land_tile(z_base, z_stand, z_top, tile.num(), tiledata.get_land_tile(tile.num()), walkable);

        WorldTile {
            tile,
            shape,
        }
    }

    /// adds to `result` all static objects located in the specified tile
    pub fn query_tile_static(&self, x: isize, y: isize, walkable: u32, ignore: u32, result: &mut Vec<WorldTile>) {
        let (idx, (ox, oy)) = self.base.tile_to_block_offsets(x, y);
        let tiledata = &self.data.tiledata;

        let statics = self.base.statics.statics_block_tile(idx, ox as u8, oy as u8);
        for static_tile in statics {
            let obj = WorldTile {
                tile: TileType::ObjectTile(static_tile.static_tile),
                shape: TileShape::from_static_tile(static_tile.z, tiledata.get_static_tile(static_tile.static_tile), walkable, ignore),
            };
            result.push(obj);
        }
    }

    /// adds to `result` all dynamic (game) objects located in the specified tile
    pub fn query_tile_dynamic(&self, x: isize, y: isize, walkable: u32, ignore: u32, result: &mut Vec<WorldTile>) {
        let (idx, (_ox, _oy)) = self.base.tile_to_block_offsets(x, y);
        let tiledata = &self.data.tiledata;

        let overlay = self.read_overlay();
        if let Some(block) = overlay.get(&idx) {
            let min_item = DynamicWorldObject::min_item(x, y);
            let max_item = DynamicWorldObject::max_item(x, y);

            for item in block.range(min_item..=max_item) {
                match item {
                    DynamicWorldObject::MultiPart { tile, z, .. } |
                    DynamicWorldObject::GameObject { graphic: tile, z, .. } => {
                        if tile & 0x30000 != 0 {
                            continue    // skip multi-objects
                        }
                        let obj = WorldTile {
                            tile: TileType::ObjectTile(*tile as u16),
                            shape: TileShape::from_static_tile(*z, tiledata.get_static_tile(*tile as u16), walkable, ignore),
                        };
                        result.push(obj);
                    }

                }
            }
        }
    }

    pub fn is_tile_multi_occupied(&self, x: isize, y: isize) -> bool {
        let (idx, (_ox, _oy)) = self.base.tile_to_block_offsets(x, y);

        let overlay = self.read_overlay();
        if let Some(block) = overlay.get(&idx) {
            let min_item = DynamicWorldObject::min_item(x, y);
            let max_item = DynamicWorldObject::max_item(x, y);

            for item in block.range(min_item..=max_item) {
                match item {
                    DynamicWorldObject::MultiPart { .. } => return true,
                    _ => continue
                }
            }
        }

        false
    }


    /// adds to `result` all objects in the given tile, and sorts them by z and height
    /// in fact it just calls query_tile_ground, query_tile_static and query_tile_dynamic and sorts `result`
    pub fn query_tile_full(&self, x: isize, y: isize, direction: u8, walkable: u32, ignore: u32, result: &mut Vec<WorldTile>) {
        result.push(self.query_tile_ground(x, y, direction, walkable));
        self.query_tile_static(x, y, walkable, ignore, result);
        self.query_tile_dynamic(x, y, walkable, ignore, result);

        result.sort_by(|a,b| {
            a.z_top()
                .cmp(&b.z_top())
                .then(a.z_base().cmp(&b.z_base()))
            // a.z_base()
            //     .cmp(&b.z_base())
            //     .then(a.z_top().cmp(&b.z_top()))
        })
    }

    /// searches for game objects in the specified area. Parts of multi-objects are ignored
    pub fn query_area_dynamic(&self, world: u8, left: isize, top: isize, right: isize, bottom: isize, result: &mut Vec<Item>) {
        let overlay = self.read_overlay();

        for xb in (left/8)..=(right/8) {
            for yb in (top/8)..=(bottom/8) {
                let idx = self.base.block_index(xb, yb);
                let block = overlay.get(&idx);

                if let Some(block) = block {
                    for item in block {
                        match item {
                            &DynamicWorldObject::GameObject { x, y, z, serial, graphic } => {
                                if x >= left && y >= top && x < right && y < bottom {
                                    result.push(Item { world, serial, graphic, x, y, z, timestamp: None, });
                                }
                            }

                            DynamicWorldObject::MultiPart { .. } => continue,
                        }
                    }
                }
            }
        }
    }
}

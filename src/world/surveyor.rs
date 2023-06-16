use std::cmp::{Ordering};
use std::collections::{BinaryHeap, HashMap};
use std::collections::hash_map::{Entry};
use std::time::Instant;
use log::{debug, info, warn};

use crate::http::server::{DistanceFunc, Point, TraceOptions};
use crate::world::{DynamicWorld, TileShape, WorldTile};


#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
struct Position (isize, isize, i8);


// fval, gval, dir, dst, src
struct ScoredPosition (isize, isize, u8, Position, Position);

impl PartialEq for ScoredPosition {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ScoredPosition {
}

impl Ord for ScoredPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}


impl PartialOrd for ScoredPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct WorldSurveyor<'a> {
    pub model: &'a DynamicWorld,
}


impl<'a> WorldSurveyor<'a> {
    /// returns a new object
    pub fn new(model: &'a DynamicWorld) -> Self {
        Self {
            model,
        }
    }

    /// returns a vector of elements located at the given coordinates and used in movement testing
    pub fn get_tile_objects(&self, x: isize, y: isize, direction: u8, result: &mut Vec<WorldTile>) {
        self.model.query_tile_full(x, y, direction, result);
    }

    fn get_top_cover(&self, x: isize, y: isize, z: i8) {
        let mut tiles = Vec::with_capacity(16);
        self.get_tile_objects(x, y, 0, &mut tiles);
        // let mut max_z = 127;
    }

    /// returns the lower and upper points of the height range reachable from the current position
    pub fn get_source_step_range(&self, x: isize, y: isize, z: i8, exit_direction: u8) -> (i8, i8) {
        let mut tiles = Vec::with_capacity(16);
        self.get_tile_objects(x, y, exit_direction, &mut tiles);
        let mut z_low_fall = i8::MIN; // lowest available point
        let mut z_high = z;     // highest available point

        for tile in &tiles {
            let (z_base, z_stand, z_top, slope) = match tile.shape {
                TileShape::Surface { z_base, z_stand, .. } =>  (z_base, z_stand, z_stand, false),
                TileShape::Slope { z_base, z_stand, z_top, .. } => (z_base, z_stand, z_top, true),
                TileShape::Background { .. } => continue,
            };

            // if the surface is lower than our position and higher than
            // the lowest point found, then update the lowest point
            if z_stand <= z && z_stand > z_low_fall {
                z_low_fall = z_stand;
            }

            // if we stand exactly on the stairs, then we try to expand
            // the available range by the boundaries of the stairs
            if slope && z_stand == z {
                z_low_fall = z_low_fall.min(z_base);
                z_high = z_high.max(z_top);
            }
        }

        const CLIMB_HEIGHT: i8 = 2;
        (z_low_fall, z_high + CLIMB_HEIGHT)
    }


    /// returns the new z position at the specified point
    /// if there is no matching position, returns None
    pub fn get_dest_position(&self, x: isize, y: isize, z: i8, z_low: i8, z_high: i8) -> Option<i8> {
        let mut objects = Vec::with_capacity(16);
        self.get_tile_objects(x, y, 0, &mut objects);    // direction doesn't matter now.
        objects.push(WorldTile::cap_tile());

        let z = if z < z_low { z_low } else { z };
        let z_high = z_high as i16;
        let mut z_low = z_low as i16;           // the lowest point where we can get on the current iteration
        let mut current_z = i8::MIN as i16;     // the highest point we have seen before current iteration

        let mut result = None::<i8>;

        for (i, upper_obj) in objects.iter().enumerate() {
            let (upper_obj_z_base, upper_obj_z_stand) = match upper_obj.shape {
                TileShape::Slope { z_base, z_stand, .. } => (z_base as i16, z_stand as i16),
                TileShape::Surface { z_base, z_stand, .. } => (z_base as i16, z_stand as i16),
                TileShape::Background { .. } => continue,
            };

            // if z_low > z_high {  // is this correct?
            //     break
            // }

            const CHARACTER_HEIGHT: i16 = 16;
            // character can fit between upper_obj_z_base and z_low
            if upper_obj_z_base - z_low >= CHARACTER_HEIGHT {
                // check the tiles below in reverse order
                for bottom_obj in objects[..i].iter().rev() {
                    let (bottom_obj_z_stand, passable) = match bottom_obj.shape {
                        TileShape::Slope { z_stand, passable,.. }   => (z_stand as i16, passable),
                        TileShape::Surface { z_stand, passable, .. } => (z_stand as i16, passable),
                        TileShape::Background { .. } => continue,
                    };

                    // if bottom_obj_z_stand < z_low {
                    //     break
                    // }

                    // if the tile is walkable, it is higher than the last viewed "upper" tile and
                    // there is enough room for the character to stand between it and the upper_tile_z_base
                    if passable && bottom_obj_z_stand >= current_z && (upper_obj_z_base - bottom_obj_z_stand) >= CHARACTER_HEIGHT {
                        // check if we can reach it from our z_high, given the type of the object
                        if !match bottom_obj.shape {
                            TileShape::Slope { z_base, .. }   => z_base as i16 <= z_high,
                            TileShape::Surface { z_stand, .. } => z_stand as i16 <= z_high,
                            TileShape::Background { .. } => unreachable!(),
                        } {
                            continue
                        };


                        // if the found position is better, remember it
                        if let Some(best_z) = result {
                            let curr_delta = (z_low - bottom_obj_z_stand).abs();
                            let prev_delta = (z - best_z).abs() as i16;

                            if curr_delta < prev_delta {
                                result = Some(bottom_obj_z_stand as i8);
                            }
                        } else {
                            result = Some(bottom_obj_z_stand as i8);
                        }
                    }
                }
            }

			// update variables if necessary
            z_low = z_low.max(upper_obj_z_stand);
			current_z = current_z.max(upper_obj_z_stand);
        }

        result
    }

    #[inline]
    pub fn direction(dx: isize, dy: isize) -> u8 {
        fn signum(n: isize) -> i32 {
            if n > 0 {
                1
            } else if n < 0 {
                -1
            } else {
                0
            }
        }
        match (signum(dx), signum(dy)) {
            (0, -1) => 0u8,
            (1, -1) => 1,
            (1, 0) => 2,
            (1, 1) => 3,
            (0, 1) => 4,
            (-1, 1) => 5,
            (-1, 0) => 6,
            (-1, -1) => 7,
            _ => unreachable!(),
        }
    }

    /// shifts the given coordinates in the specified direction
    #[inline]
    pub fn move_to(x: isize, y: isize, direction: u8) -> (isize, isize) {
        match direction & 7 {
            0 => (x+0, y-1),
            1 => (x+1, y-1),
            2 => (x+1, y+0),
            3 => (x+1, y+1),
            4 => (x+0, y+1),
            5 => (x-1, y+1),
            6 => (x-1, y+0),
            7 => (x-1, y-1),
            _ => unreachable!(),
        }
    }

    /// turns the direction by the specified number of steps, can turn both to the right,
    /// with positive steps, and to the left, with negative steps.
    /// If steps is null, returns the original value
    #[inline]
    fn turn_to(direction: u8, steps: i8) -> u8 {
        (direction as i8 + steps).rem_euclid(8) as u8
    }

    /// just checks if it is possible to step from the starting position in the specified direction
    fn test_step_single(&self, x: isize, y: isize, z: i8, direction: u8) -> Option<i8> {
        let (to_x, to_y) = Self::move_to(x, y, direction);
        let (z_low, z_high) = self.get_source_step_range(x, y, z, direction);
        self.get_dest_position(to_x, to_y, z, z_low, z_high)
    }

    /// a more complete check of the movement, additionally checking
    /// the passability of adjacent tiles for the diagonal direction of the step
    pub fn test_step(&self, x: isize, y: isize, z: i8, direction: u8) -> Option<i8> {
        // check destination tile
        let dest_z = self.test_step_single(x, y, z, direction)?;

        // if not diagonal direction just return result
        if direction & 1 == 0  {
            return Some(dest_z)
        }

        // check adjacent tiles for diagonal step
        // right tile
        self.test_step_single(x, y, z, Self::turn_to(direction, 1))?;

        // left tile
        self.test_step_single(x, y, z, Self::turn_to(direction, -1))?;

        // adjacent tiles ok, return destination z
        Some(dest_z)
    }


    /// searches for a path by algorithm A* from the point s_x,s_y,s_z to the point d_x, d_y, d_z.
    /// `points` will contain the found path to the nearest possible point, or all points explored during the search,
    /// depending on the options.
    /// Also, through `options`, you can fine-tune the parameters of the algorithm, such as the distance function,
    /// heuristic coefficients, boundaries of the path search area.
    pub fn trace_a_star(&self, s_x: isize, s_y: isize, s_z: i8, sdir: u8, d_x: isize, d_y: isize, d_z: i8, ddir: u8, points: &mut Vec<Point>, options: &TraceOptions) {
        let mut cached_steps = HashMap::new();
        let mut frontier = BinaryHeap::new();
        let mut visited = HashMap::new();
        let mut back_path = HashMap::new();

        let x_accuracy = options.accuracy_x.unwrap_or(0);
        let y_accuracy = options.accuracy_y.unwrap_or(0);
        let z_accuracy = options.accuracy_z.unwrap_or(0);

        let cost_limit = options.cost_limit.unwrap_or(isize::MAX);
        let cost_turn = options.cost_turn.unwrap_or(1);
        let cost_move_straight = options.cost_move_straight.unwrap_or(1);
        let cost_move_diagonal = options.cost_move_diagonal.unwrap_or(cost_move_straight);
        let allow_diagonal_move = options.allow_diagonal_move.unwrap_or(false);

        let h_dist = options.heuristic_distance.unwrap_or(DistanceFunc::Diagonal);
        let h_direct = options.heuristic_straight.unwrap_or(5);
        let h_diagonal = options.heuristic_diagonal.unwrap_or(h_direct);

        let left = options.left.unwrap_or(0);
        let top = options.top.unwrap_or(0);
        let right = options.right.unwrap_or(self.model.base.width() as isize);
        let bottom = options.bottom.unwrap_or(self.model.base.height() as isize);

        let all_points = options.all_points.unwrap_or(false);

        let dist_func = |dx: isize, dy: isize| {
            match h_dist {
                DistanceFunc::Manhattan => (dx + dy) * h_direct,
                DistanceFunc::Chebyshev => dx.max(dy) * h_direct,
                DistanceFunc::Diagonal => h_direct * (dx + dy) + (h_diagonal - 2 * h_direct) * dx.min(dy),
                DistanceFunc::Euclidean => f64::sqrt((dx * dx + dy * dy) as f64) as isize * h_direct,
            }
        };

        let h_func = |position: &Position| {
            let dx = (d_x -position.0).abs();
            let dy = (d_y -position.1).abs();
            dist_func(dx, dy)
        };


        let check_step = |x: isize, y: isize, z: i8, dir: u8, cache: &mut HashMap<(isize, isize, i8, u8), Option<i8>>| {
            let (dx, dy) = Self::move_to(x, y, dir);

            if dx < left || dx >= right || dy < top || dy >= bottom { // check bounds
                return None
            }

            match cache.entry((x, y, z, dir)) {
                Entry::Occupied(entry) => {
                    *entry.get()
                }

                Entry::Vacant(entry) => {
                    let result = self.test_step_single(x,y,z, dir);

                    entry.insert(result);
                    result
                }
            }
        };

        let start_pos = Position(s_x, s_y, s_z);
        let start_gval = 0;
        let start_fval = start_gval + h_func(&start_pos);
        let scored_start_pos = ScoredPosition(start_fval, start_gval, sdir, start_pos, Position(-1, -1, -1));
        frontier.push(scored_start_pos);

        let start_time = Instant::now();
        let mut cnt = 0;

        let mut best_dist = isize::MAX;
        let mut best_pos = None;

        while let Some(curr_scored_pos) = frontier.pop() {
            let ScoredPosition(curr_fval, curr_gval, curr_dir, curr_pos, src_pos) = curr_scored_pos;
            let Position(curr_x, curr_y, curr_z) = curr_pos;

            cnt += 1;
            if cnt % 100000 == 0 {
                debug!("{cnt} ({curr_x} {curr_y}) current score {curr_fval}, frontier len {}", frontier.len());
            }

            // check if the current position has been visited before
            match visited.entry(curr_pos) {
                Entry::Occupied(_) => { // already visited
                        continue;
                }
                Entry::Vacant(entry) => {   // not visited
                    entry.insert(curr_gval);
                }
            }

            back_path.insert(curr_pos, src_pos);

            // goal check
            let d_x = (d_x - curr_x).abs();
            let d_y = (d_y - curr_y).abs();
            let d_z = (d_z - curr_z).abs() as isize;
            let d_max = d_x.max(d_y).max(d_z);

            if d_max < best_dist {
                best_pos = Some(curr_pos);
                best_dist = d_max;
            }

            if d_x <= x_accuracy && d_y <= y_accuracy && d_z <= z_accuracy {
                info!("Found! {curr_x} {curr_y} {curr_gval} {curr_fval}");
                break
            }

            let dest_n = check_step(curr_x, curr_y, curr_z, 0, &mut cached_steps);
            let dest_e = check_step(curr_x, curr_y, curr_z, 2, &mut cached_steps);
            let dest_s = check_step(curr_x, curr_y, curr_z, 4, &mut cached_steps);
            let dest_w = check_step(curr_x, curr_y, curr_z, 6, &mut cached_steps);

            let steps = if allow_diagonal_move {
                let dest_ne = if dest_n.is_some() && dest_e.is_some() { check_step(curr_x, curr_y, curr_z, 1, &mut cached_steps) } else { None };
                let dest_se = if dest_s.is_some() && dest_e.is_some() { check_step(curr_x, curr_y, curr_z, 3, &mut cached_steps) } else { None };
                let dest_sw = if dest_s.is_some() && dest_w.is_some() { check_step(curr_x, curr_y, curr_z, 5, &mut cached_steps) } else { None };
                let dest_nw = if dest_n.is_some() && dest_w.is_some() { check_step(curr_x, curr_y, curr_z, 7, &mut cached_steps) } else { None };

                [(0, dest_n), (1, dest_ne), (2, dest_e), (3, dest_se), (4, dest_s), (5, dest_sw), (6, dest_w), (7, dest_nw)]
            } else {
                [(0u8, dest_n),  (2, dest_e), (4, dest_s), (6, dest_w), (1, None), (3, None), (5, None), (7, None)]
            };

            for (direction, dest_result) in steps {
                if let Some(dest_z) = dest_result {
                    let (dest_x, dest_y) = Self::move_to(curr_x, curr_y, direction);
                    let dest_pos = Position(dest_x, dest_y, dest_z);

                    match visited.entry(dest_pos) {
                        Entry::Occupied(_) => {
                            continue;
                        }
                        Entry::Vacant(_) => {}
                    }

                    let dest_gval = curr_gval + if direction & 1 != 0 {
                        if direction == curr_dir { cost_move_diagonal } else { cost_move_diagonal + cost_turn }
                    } else {
                        if direction == curr_dir { cost_move_straight } else { cost_move_straight + cost_turn }
                    };

                    if dest_gval > cost_limit {
                        continue
                    }

                    let dest_fval = dest_gval + h_func(&dest_pos);
                    let dest_scored = ScoredPosition(dest_fval, dest_gval, direction, dest_pos, curr_pos);

                    frontier.push(dest_scored);
                }
            }
        }

        let duration = start_time.elapsed();
        debug!("total tiles explored {cnt} and visited {} in {:?}", visited.len(), duration);

        if all_points {
            for (Position(x, y,z), w) in visited {
                points.push(Point{ x, y, z, w, });
            }
        } else if let Some(mut curr_pos) = best_pos {
            cnt = 0;
            info!("search path to start from {curr_pos:?} with score {best_dist}");
            loop {
                cnt += 1;
                let prev_pos = back_path[&curr_pos];
                points.push(Point{ x: prev_pos.0, y: prev_pos.1, z: prev_pos.2, w: 0, });

                if prev_pos == start_pos {
                    info!("found start, path len is {cnt} tiles!");
                    break;
                }
                curr_pos = prev_pos;
            }
            points.reverse();
        } else {
            warn!("there is no data to return after tracing completes")
        }
    }
}

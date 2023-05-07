# HTTP API

URL http://127.0.0.1:3000/api/
METHOD POST
Content-type application/json

## World state control
### Save
{"WorldSave": {"file_name": file_name}}
->
{"Success":{}}

Saves the current state to the specified file
    

### Load
{"WorldLoad": {"file_name": file_name}}
->
{"Success":{}}

Loads the state of the world from the specified file
    

### Clear
{'WorldClear': {}}
->
{"Success":{}}

Completely clears the world of dynamic items



## Items management
### Add items
{"ItemsAdd": {"items": [{"world": u8, serial": u32, "x": isize, "y": isize, "z": i8, "graphic": u16}, ...]}}
->
{"Success":{}}

Adds an items with the specified parameters to the world. 
If an item with that `serial` already exists, then the old copy will be deleted.
Only one item with a unique `serial` can exist in the world.

if the object is a multi-object, then its parts are added to the world.


### Delete item
{"ItemsDel": {"serials": [u32, ...]}}
->
{"Success":{}}

Removes the item with the specified `serials` from the world.

if the object is multi-object, then all its parts will be removed from the world.

## Querying
TODO    

Searches for items in the specified area.

The query will not return parts of the multi-object, only the object itself.

## Pathfinding

### Search the Path
{"TracePath": 
    {"world": u8, 
     "sx": isize, "sy": isize, "sz": i8, 
     "dx": isize, "dy": isize, "dz": i8,
     "options": {...}
} -> [{"x": isize, "y": isize, "z": i8, "w": isize}, ... ]

Searches for a path from the specified start point (sx,sy,sz) to the end point (dx,dy,dz), 
taking into account options, returns the path found or empty if it is impossible to move at all.
Using `options` allows you to set additional options for finding the path.

Returns a list of coordinates that can be used to reach the nearest point to the target

    
### Explore
{"TraceArea": {"world": world, "x": x, "y": y, "z": z, "options": {}}}
->
[{"x": isize, "y": isize, "z": i8, "w": isize}, ... ]

From the starting point (x,y,z) explores the area, given the options from `options`
Returns an unordered list of all reachable points with additional information.
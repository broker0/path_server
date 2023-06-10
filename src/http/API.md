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
{"ItemsAdd": {"items": [{"world": u8, serial": u32, "x": isize, "y": isize, "z": i8, "graphic": u32}, ...]}}
->
{"Success": {}}

Adds an items with the specified parameters to the world. 
If the item is standard multi-object, then `graphic` must have the flag 0x10000.
If an item with that `serial` already exists, then the old copy will be deleted.
Only one item with a unique `serial` can exist in the world.

if the object is a multi-object, then its parts are added to the world.


### Custom Houses
{"MultiItemsAdd": 
    [
        {"world": u8, 
         "serial": u32, 
         "x": isize, 
         "y": isize, 
         "z": u8, 
         "graphic": u32, 
         "parts": [
            {"x": isize, 
             "y": isize, 
             "z": i8, 
             "graphic": u16"
            },
            ...
         ]
        },
        ...
    ]
}
->
{"Success": {}}

Adds custom houses to the world. Field `graphic` must have flag 0x20000.
Works like `ItemsAdd`, but all parts of the house will be sended and added, not only main object.
Essentially, the source of the parts of the multi-object will be the data from the request, not the data loaded from the multi.mul.
Deleting a multi-object with `ItemsDel`.


### Delete item
{"ItemsDel": {"serials": [u32, ...]}}
->
{"Success": {}}

Removes the item with the specified `serials` from the world.

if the object is multi-object, then all its parts will be removed from the world.


## Querying
{"Query": {"left": isize, "top": isize, "right": isize, "bottom": isize}}
->
{"QueryReply": {"items": [{"world": u8, serial": u32, "x": isize, "y": isize, "z": i8, "graphic": u16, "timestamp": u64}, ...]}}

Searches for items in the specified area.

The response will include items with coordinates
`left <= item.x < right`
`top <= item.y < bottom`

In general, the response contains the same that was added using `ItemsAdd`, 
but includes an additional field `timestamp` - the time the item was last updated.

The query will not return parts of the multi-object, only the game objects.

Also for multi-objects the `graphic` field will be set to 0x0000.


## Pathfinding

### TraceOptions
Allows you to fine-tune the path search algorithm by setting the cost of movements, 
the heuristics function, limit the search area.
All fields are optional and allow you to change the default settings.

{
    "left": isize, "top": isize, "right": isize, "bottom": isize,
    "accuracy_x": isize, "accuracy_y": isize, "accuracy_z": isize,
    "cost_turn": isize, "cost_move_straight": isize, "cost_move_diagonal": isize,
    "heuristic_distance": isize, "heuristic_straight": isize, "heuristic_diagonal": isize,
    "all_points": isize, "open_doors": isize, "allow_diagonal_move": isize, "cost_limit": isize
}

#### Explanation of options

`left`, `top`, `right`, `bottom` - the boundaries of the search area. 
Default values are current world dimensions.

`accuracy_x`, `accuracy_y`, `accuracy_z` - the accuracy of finding the end point of the path.
Default value is 0.

`allow_diagonal_move` - allows you to enable or disable diagonal movement.
Moving diagonally allows you to find better paths, but at the cost of slowing down twice, 
because each step has to check not 4 possible directions, but 8.
Default value is `false`.

 `cost_turn`, `cost_move_straight`, `cost_move_diagonal` - cost of moving and turning (changing direction).
The default value is 1. 
 If the value of `cost_move_diagonal` is not set explicitly, it will be equal to `cost_move_straight`.
 
The cost can be set to anything - for example, the movement time in milliseconds:
`cost_turn`=50, `cost_move_straight`=100, `cost_move_diagonal`=100

Or you can consider them as a penalty for certain actions, higher value, less preferable action:
`cost_turn`=100, `cost_move_straight`=25, `cost_move_diagonal`=50

The higher cost of turns and diagonal movement will lead to the fact that 
a route will be built that meets these requirements.

`cost_limit` - allows you to set limit of the cost, but for this limit to work, `cost_` values must be greater than 0
Default value somewhere around `INT_MAX`


`heuristic_distance` - specifies a function that measures the distance between two points "in a straight line".
Default value is "Diagonal".

Can be one of these string values: "Manhattan", "Chebyshev", "Diagonal", "Euclidean".

`dx` = `abs`(`dest_x` - `curr_x`), `dy` = `abs`(`dest_y` - `curr_y`)

If it's quite simply "Manhattan" considers the greater of `dx`, `dy` as a distance.

"Euclidean" - the most common Euclidean distance is `sqrt`(`dx`*`dx`+`dy`*`dy`). In the world of Ultima, 
it doesn't make much sense, and is rather expensive to calculate due to the int->float-sqrt->int conversion. 
But it allows you to find a little more “smooth” and not so diagonal paths.

"Chebyshev" - the distance to the point is the sum of `dx`+`dy`. 

"Diagonal" - the distance is determined jointly from the "straight" and "diagonal" components with their own cost, according to a rather tricky formula.

If diagonal movement is enabled, "Diagonal" is usually the best choice.
If diagonal movement is not required, then "Manhattan" is preferable.


`heuristic_straight`, `heuristic_diagonal` - a heuristic coefficient that determines the "strength" with which the endpoint "attract to itself".
If the value of `heuristic_diagonal` is not set explicitly, it will be equal to `heuristic_straight`.
If set to 0, then the path search will have no preference in the direction of tile exploring, and a breadth-first search will be performed instead of A*.
The higher the value, the faster the path will be found, but at the cost of worsening optimality.
`heuristic_diagonal` is only used if `heuristic_distance` set to "Diagonal".
Default value is 5.


`all_points` - if set to `true`, then the result of the path search will include not only the path, but also all explored points in random order. 
This allows you to explore a certain area and get all the tiles available in it.
Default value is `false`


Options not described most likely do not work.


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

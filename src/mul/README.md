# Description of files format

All data in files no aligned, packed. Byte order in all files is little-endian.

The described structures only describe the file format, and may not be represented in the code, 
a kind of pseudocode.

## Common structures

```rust
#[repr(C, packed)]
struct LookupIndexRecord {
    offset: u32,    // must be multiple of the size StaticTile
    length: u32,    // also
    unknown1: u32,  // unknown
}
```

This structure contain information about variable data block in related file.
Offset is offset from file begin (must be multiple of size element)
Length is length block in bytes (must be multiple of size element)
Unknown - unknown field, possible checksum or unknown flags, doesn't look like random garbage


## Tiledata

Data stored in single file `tiledata.mul`. File virtually splitted to two parts - 
land tiles description and static tile description.

Land tiles is base tiles of world - map surface.

Static tiles is any game objects, but static, non-movable. In most cases it is tree, 
foliage and most parts of building. And more other types of objects. 

First part of file has fixed size, size of second part calculated from file size.


### LandTile
Format of LandTile described bellow:
```rust
// size in bytes 4+2+20=26 bytes
#[repr(C, packed)]
struct LandTile {
    flags: u32,
    texture_id: u16,
    tile_name: [u8; 20],
}
```

### LandTileGroup
LandTile's packed to groups of 32 elements with unknown header, possible checksum. Format described bellow:

```rust
// size in bytes 4+32*26=836 byte
#[repr(C, packed)]
struct LandTileGroup {
    header: u32,
    tiles: [LandTile; 32],
}
```

File has 512 LandTileGroup elements. Total 16384 LandTile's.

Second part of file looks similar - StaticTile's packed to groups of 32 elements. 
But  number of groups must be calculated from file size.


### StaticTile
StaticTile keep more information about tile thad LandTile. Because describes any game items and
describes many properties of item - stackable, weight, many flags about item role, etc.

```rust
// size in bytes 4+1+1+2+1+1+2+1+1+2+1+20=37 bytes
#[repr(C, packed)]
struct StaticTile {
    flags: u32,
    weight: u8,
    quality: u8,
    unk1: u16,
    unk2: u8,
    quantity: u8,
    anim_id: u16,
    unk3: u8,
    hue: u8,
    unk4: u16,
    height: u8,
    tile_name: [u8; 20],
}
```


### StaticTileGroup
StaticTile's packed to groups of 32 elements with unknown header, possible checksum. Format described bellow:

```rust
// size in bytes 4+32*37=1188
#[repr(C, packed)]
struct StaticTileGroup {
    header: u32,
    tiles: [StaticTile; 32],
}
```

## Map

Map files contain information about LandTiles position (altitude) - `MapTile` grouped by blocks 8x8 tiles.
Each group has unknown header, possible checksum - `MapBlock`.

Groups stored in file as a two-dimensional array with dimensions depending on map number(?). 
For example used size of basic worlds (0-1) - 6144x4096 tiles or 768x512 groups.

Array stored by columns, not rows.  Index of block by x,y MapBlock calculated as `index = x * 512[HEIGHT] + y`

```
Map blocks order in the file:

+---+---+---+--
|  0|512|   |  
+---+---+---+--
|  1|513|   |  
+---+---+---+--
|  2|   |   |  
 ...
```

```
Tiles order in map block 8x8

+---+---+---+--
|  0|  1|  2|... 
+---+---+---+--
|  8|  9| 10|  
+---+---+---+--
|   |   |   |   

```


### MapTile

```rust
#[repr(C, packed)]
struct MapTile {
    land_tile: u16,
    z: i8,
}
```

### MapBlock
```rust
#[repr(C, packed)]
struct MapBlock {
    header: u32,
    cells: [[MapTile; 8]; 8],
}
```

### Map0
```rust
#[repr(C, packed)]
struct Map0 {
    blocks: [[MulMapBlock; 512]; 768],
}
```

## Static

Statics data is stored in two files - staidx<N>.mul and statics<N>.mul
In file staidx<N> stores information about statics tiles in map block.
if field offset equal to 0xFFFFFFFF it means that this block has no static tiles

File contain fixed (depending on map number) number of `LookupIndexRecord` grouped as 2d array by columns.

In file statics<N>.mul stores information about `StaticTile` positions.
StaticTile's grouped to blocks related to map block. Count of tiles can be variable. 
Tiles has relative offset from block origin.


### StaticIndex

```rust
#[repr(C, packed)]
struct StaticIdx {
    blocks: [[LookupIndexRecord; 512]; 768],
}
```


### StaticTile

```rust
#[repr(C, packed)]
struct StaticTile {
    static_tile: u16,
    x: u8,  // offset inside block 0..7
    y: u8,
    z: i8,
    unknown1: u16,
}
```


## Multi

Multi-objects also stored in two file - index `multi.idx` and data `multi.mul`.

Index file contain array of `LookupIndexRecord`. Count of records calculated from file size.
Data file contain parts of multi-object.


### MultiTile

```rust
#[repr(C, packed)]
pub struct MultiTile {
    static_tile: u16,
    x: u16,
    y: u16,
    z: u16,
    flags: u32,
}
```
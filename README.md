# Pathfinding Server for Ultima Online

## Short description

This program is designed to finding paths in Ultima Online.

It allows you to build paths taking into account various game items.

Using the http json api at <http://127.0.0.1/api/> you can add this data about these objects using clients such as Orion or Stealth, with the ability to save and load the state of the world later.

Of course, api provides the ability to calculate paths between any point in the world, with the ability to fine-tune the parameters.

And additionally, you can render part or all of the world map and display the path found on it in png format.

There is a simple graphical interface for exploring the world, controlled by `arrows`, `ctrl` and `left shift` keys.

![screen shot](examples/screenshot.png "Title")

In addition, there is web-ui available at  <http://127.0.0.1:3000/ui/>

## Building and launch

Сompilation takes place with the usual cargo build -r
you can also run cargo run -r

The current directory must contain the following data files

Radarcol.mul
Statics0.mul
Map0.mul
Staidx0.mul
Map2.mul
Statics2.mul
Staidx2.mul
multi.idx
multi.mul
tiledata.mul

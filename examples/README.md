Here are some examples of using the `path_server`:

`orion_client.oajs` script for client Ultima Online - Orion. 
The script collects information about items around the player, forms a list and sends it to `path_server` to update the world.

`api.py` is the simplest implementation of api requests, there is no error checking.

`stealth_client.py` is a very simple example for the Stealth Client of moving along the found path.

`usage.py` is a simple example making several calls using the api from the file above.
Performs a call to load and clear the world.
Searches for a path between two distant points and prints its length, additionally configuring the tracer options.
Renders a full map, overlaying the found path on it and saves it to the `map.png` file


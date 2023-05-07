from api import PathApi
import time

api = PathApi("http://127.0.0.1:3000/api/")

api.WorldClear()
api.WorldSave("test.save")
reply = api.WorldLoad('test.save')
print(reply)
api.WorldClear()

start = time.time()
# pathfinding options
api.options.heuristic_distance = "Diagonal"
api.options.heuristic_straight = 1000
api.options.allow_diagonal_move = True
api.options.all_points = False

reply = api.TracePath(0, 2151, 3657, 0, 3424, 104, 0)
assert('TraceReply' in reply)
path = reply["TraceReply"]["points"]
print("length of path: ", len(path))
print(f"time of path search {time.time()-start}")


start = time.time()
reply = api.RenderArea(0, 0, 0, 6144, 4096, path, color=0xFFFFFF)
with open("map.png", "wb") as fl:
    fl.write(reply)
print(f"time of map rendering {time.time()-start}")

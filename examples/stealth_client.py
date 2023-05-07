import stealth as st
from api import PathApi

api = PathApi("http://127.0.0.1:3000/api/")
api.options.heuristic_distance = "Diagonal"
api.options.allow_diagonal_move = True
api.options.accuracy_x = 10
api.options.accuracy_y = 10
api.options.accuracy_z = 100


def get_path(dest_x, dest_y):
    x, y, z = st.GetX(st.Self()), st.GetY(st.Self()), st.GetZ(st.Self())
    reply = api.TracePath(st.WorldNum(), x, y, z, dest_x, dest_y, z)
    assert ('TraceReply' in reply)
    path = reply["TraceReply"]["points"]
    print("length of path: ", len(path))
    return path


def walk(path):
    for point in path:
        st.MoveXYZ(point["x"], point["y"], point["z"], 0, 0, True)


walk(get_path(1888, 1476))

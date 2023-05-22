import requests


class PathApi:
    def __init__(self, api_url):
        self.url = api_url
        self.options = TraceOptions()

    def WorldSave(self, file_name):
        request = {'WorldSave': {'file_name': file_name}}
        return self.api_request(request)

    def WorldLoad(self, file_name):
        request = {"WorldLoad": {"file_name": file_name}}
        return self.api_request(request)

    def WorldClear(self):
        request = {'WorldClear': {}}
        return self.api_request(request)

    def ItemsAdd(self, items):
        request = {"ItemsAdd": {"items": items}}
        return self.api_request(request)

    def ItemsDel(self, serials):
        request = {"ItemsDel": {"serials": serials}}
        return self.api_request(request)

    def TracePath(self, world, sx, sy, sz, dx, dy, dz):
        request = {"TracePath": {"world": world, "sx": sx, "sy": sy, "sz": sz, "dx": dx, "dy": dy, "dz": dz,
                                 "options": self.options.opts()}}
        return self.api_request(request)

    def RenderArea(self, world, left, top, right, bottom, points, color=None):
        request = {
            "RenderArea": {"world": world, "points": points, "color": color, "left": left, "top": top, "right": right,
                           "bottom": bottom}}
        return self.api_request(request)

    def api_request(self, request):
        reply = requests.post(self.url, json=request)
        if "RenderArea" in request:
            return reply.content
        else:
            return reply.json()


class TraceOptions:
    def __init__(self,
                 left=None, top=None, right=None, bottom=None,
                 accuracy_x=None, accuracy_y=None, accuracy_z=None,
                 cost_turn=None, cost_move_straight=None, cost_move_diagonal=None,
                 heuristic_distance=None, heuristic_straight=None, heuristic_diagonal=None,
                 allow_diagonal_move=None, all_points=None, open_door=None
                 ):
        self.left = left
        self.top = top
        self.right = right
        self.bottom = bottom

        self.accuracy_x = accuracy_x
        self.accuracy_y = accuracy_y
        self.accuracy_z = accuracy_z

        self.allow_diagonal_move = allow_diagonal_move
        self.cost_turn = cost_turn
        self.cost_move_straight = cost_move_straight
        self.cost_move_diagonal = cost_move_diagonal

        self.heuristic_distance = heuristic_distance
        self.heuristic_straight = heuristic_straight
        self.heuristic_diagonal = heuristic_diagonal

        self.all_points = all_points
        self.open_door = open_door

    def opts(self):
        return {k: v for (k, v) in self.__dict__.items() if v is not None}


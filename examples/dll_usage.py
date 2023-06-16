import ctypes

dll = ctypes.WinDLL ("./path_server_lib.dll")

dll.start_path_server.argtypes = (ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint16)
dll.start_path_server.restype = ctypes.c_bool


data_path = b"."
ui_file = b"www/ui.html"
http_port = 3000

result = dll.start_path_server(data_path, ui_file, http_port)

if result:
    input("server started, press enter to stop")
    dll.stop_path_server()
    print("server stopped")
else:
    print("server start failed")

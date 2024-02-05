import ctypes
import json
import os
import platform

LOADED_LIB_CACHE = None


def get_lib():
    global LOADED_LIB_CACHE
    if LOADED_LIB_CACHE == None:
        script_file = os.path.realpath(__file__)
        directory = os.path.dirname(script_file)
        
        system = platform.system()
        library_name = "libtimeline.dylib"  # Default to macOS
        if system == "Windows":
            library_name = "timeline.dll"
        
        rust_lib_path = os.path.join(
            directory, library_name)
        rust_lib = ctypes.cdll.LoadLibrary(rust_lib_path)
        rust_lib.call_command.restype = ctypes.c_char_p
        LOADED_LIB_CACHE = rust_lib

    return LOADED_LIB_CACHE


def call_lib(message):
    payload = json.dumps(message).encode('utf-8')
    lib = get_lib()
    json_ptr = lib.call_command(payload)
    json_str = ctypes.c_char_p(json_ptr).value.decode('utf-8')
    data = json.loads(json_str)
    lib.free_command(json_ptr)
    return data


result = call_lib({
    'command': 'connect',
    'db_path': '../data/.aaaaa.blend.timeline',
    'path_to_blend': '../data/untitled_3.blend',
    'message': "test"
})

print(result)

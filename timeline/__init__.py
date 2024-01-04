import bpy
import bpy_extras
from bpy.app.handlers import persistent
import ctypes
import os
import json
from threading import Timer
from urllib.parse import quote_plus

bl_info = {
    "name": "Blender timeline",
    "blender": (2, 80, 0),
    "category": "Object",
}

CHECKMARK_ICON = 'CHECKMARK'
BLANK_ICON = 'BLANK1'
TRIA_RIGHT = 'TRIA_RIGHT'


def get_file_path():
    return bpy.data.filepath


def get_db_path(file_path):
    filename = os.path.basename(file_path)
    filename_with_ext = filename + '.timeline'
    return os.path.join(os.path.dirname(file_path), filename_with_ext)


def save_file():
    bpy.ops.file.pack_all()
    bpy.ops.wm.save_mainfile()


def open_file(path):
    bpy.ops.wm.open_mainfile(filepath=path)


def refresh_file():
    bpy.ops.wm.revert_mainfile()


class WaitCursor():
    def __enter__(self):
        bpy.context.window.cursor_set("WAIT")

    def __exit__(self, *args, **kwargs):
        pass


LOADED_LIB_CACHE = None

CONNECTED = False
CURRENT_BRANCH = None
CURRENT_CHECKPOINT_HASH = None
CHECKPOINT_ITEMS = None
BRANCH_ITEMS = None


def set_checkpoints_from_state(context):
    global CHECKPOINT_ITEMS
    context.scene.checkpoint_items.clear()
    for checkpoint in CHECKPOINT_ITEMS:
        item = context.scene.checkpoint_items.add()
        item.hash = checkpoint[0]
        item.message = checkpoint[1]


def set_branches_from_state(context):
    global BRANCH_ITEMS
    context.scene.branch_items.clear()
    for branch in BRANCH_ITEMS:
        item = context.scene.branch_items.add()
        item.name = branch


def set_branches_checkpoints_from_state(context):
    set_branches_from_state(context)
    set_checkpoints_from_state(context)


def get_lib():
    global LOADED_LIB_CACHE
    if LOADED_LIB_CACHE == None:
        script_file = os.path.realpath(__file__)
        directory = os.path.dirname(script_file)
        rust_lib_path = os.path.join(
            directory, 'libtimeline.dylib')
        rust_lib = ctypes.cdll.LoadLibrary(rust_lib_path)
        rust_lib.call_command.restype = ctypes.c_char_p
        LOADED_LIB_CACHE = rust_lib

    return LOADED_LIB_CACHE


def unload_lib(lib):
    dlclose_func = ctypes.CDLL(None).dlclose
    dlclose_func.argtypes = [ctypes.c_void_p]
    dlclose_func.restype = ctypes.c_int
    dlclose_func(lib._handle)


def call_lib(message):
    payload = json.dumps(message).encode('utf-8')
    lib = get_lib()
    json_ptr = lib.call_command(payload)
    json_str = ctypes.c_char_p(json_ptr).value.decode('utf-8')
    data = json.loads(json_str)
    lib.free_command(json_ptr)
    return data


class ConnectOperator(bpy.types.Operator):
    """Connect to the Timeline"""
    bl_idname = "wm.connect"
    bl_label = "Connect to the Timeline"

    def execute(self, context):
        file_path = get_file_path()

        result = call_lib({
            'command': 'connect',
            'db_path': get_db_path(file_path),
            'path_to_blend': file_path
        })

        if 'error' in result:
            self.report({'ERROR'}, str(result))
            return {'FINISHED'}

        global CONNECTED
        CONNECTED = True

        global CURRENT_BRANCH
        CURRENT_BRANCH = result['current_branch_name']

        global BRANCH_ITEMS
        BRANCH_ITEMS = result['branches']

        global CURRENT_CHECKPOINT_HASH
        CURRENT_CHECKPOINT_HASH = result['current_checkpoint_hash']

        global CHECKPOINT_ITEMS
        CHECKPOINT_ITEMS = result['checkpoints_on_this_branch']

        set_branches_checkpoints_from_state(context)

        return {'FINISHED'}


class BlendFileFromTimelineOperator(bpy.types.Operator, bpy_extras.io_utils.ImportHelper):
    """Connect to the Timeline"""
    bl_idname = "wm.blend_file_from_timeline"
    bl_label = "Connect to the Timeline"

    def execute(self, context):
        db_path = str(self.filepath)

        result = call_lib({
            'command': 'blend-file-from-timeline',
            'db_path': db_path,
        })

        if 'error' in result:
            self.report({'ERROR'}, str(result))
            return {'FINISHED'}

        open_file(result['restored_file_path'])

        return {'FINISHED'}


class SwitchBranchesOperator(bpy.types.Operator):
    """Switch to another branch"""
    bl_idname = "wm.switch_branch_operator"
    bl_label = "Switch Branch"

    name: bpy.props.StringProperty(name="Branch name", default="")

    def execute(self, context):
        with WaitCursor():
            file_path = get_file_path()

            result = call_lib({
                'command': 'switch-to-branch',
                'db_path': get_db_path(file_path),
                'path_to_blend': file_path,
                'branch_name': str(self.name)
            })

            if 'error' in result:
                self.report({'ERROR'}, str(result))
                return {'FINISHED'}

            refresh_file()

            global CURRENT_BRANCH
            CURRENT_BRANCH = result['current_branch_name']

            global CURRENT_CHECKPOINT_HASH
            CURRENT_CHECKPOINT_HASH = result['current_checkpoint_hash']

            global CHECKPOINT_ITEMS
            CHECKPOINT_ITEMS = result['checkpoints_on_this_branch']

            set_branches_checkpoints_from_state(context)

            return {'FINISHED'}


class NewBranchOperator(bpy.types.Operator):
    """Create a new branch"""
    bl_idname = "wm.new_branch_operator"
    bl_label = "New Branch"

    name: bpy.props.StringProperty(name="Branch name", default="")

    def execute(self, context):
        with WaitCursor():
            file_path = get_file_path()

            result = call_lib({
                'command': 'switch-to-new-branch',
                'db_path': get_db_path(file_path),
                'branch_name': str(self.name)
            })

            if 'error' in result:
                self.report({'ERROR'}, str(result))
                return {'FINISHED'}

            global CURRENT_BRANCH
            CURRENT_BRANCH = result['current_branch_name']

            global BRANCH_ITEMS
            BRANCH_ITEMS = result['branches']

            set_branches_checkpoints_from_state(context)

            return {'FINISHED'}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


class CheckpointItem(bpy.types.PropertyGroup):
    hash: bpy.props.StringProperty(name="Hash", default="")
    message: bpy.props.StringProperty(name="Message", default="")


class BranchItem(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(name="Name", default="")


class RestoreOperator(bpy.types.Operator):
    """Restore a checkpoint"""
    bl_idname = "my.restore_operator"
    bl_label = "Restore"

    hash: bpy.props.StringProperty(name="Hash", default="")

    def execute(self, context):
        with WaitCursor():
            file_path = get_file_path()

            result = call_lib({
                'command': 'restore-checkpoint',
                'db_path': get_db_path(file_path),
                'path_to_blend': file_path,
                'hash': str(self.hash)
            })

            if 'error' in result:
                self.report({'ERROR'}, str(result))
                return {'FINISHED'}

            refresh_file()

            global CURRENT_CHECKPOINT_HASH
            CURRENT_CHECKPOINT_HASH = result['current_checkpoint_hash']

            set_branches_checkpoints_from_state(context)

            return {'FINISHED'}


class RestoreToFileOperator(bpy.types.Operator):
    """Restore a checkpoint to a new file"""
    bl_idname = "my.restore_to_file_operator"
    bl_label = "Restore to a new file"

    hash: bpy.props.StringProperty(name="Hash", default="")
    name: bpy.props.StringProperty(name="Checkpoint name", default="")

    def execute(self, context):
        with WaitCursor():
            file_path = get_file_path()
            file_name = os.path.basename(file_path)
            file_name = "_".join(self.name.split(" ")) + ".blend"
            destination_file_path = os.path.join(
                os.path.dirname(file_path), file_name)

            result = call_lib({
                'command': 'restore-checkpoint',
                'db_path': get_db_path(file_path),
                'path_to_blend': destination_file_path,
                'hash': str(self.hash)
            })

            if 'error' in result:
                self.report({'ERROR'}, str(result))

            return {'FINISHED'}


class CreateCheckpointOperator(bpy.types.Operator):
    """Create a new checkpoint"""
    bl_idname = "my.create_checkpoint_operator"
    bl_label = "Create Checkpoint Operator"

    def execute(self, context):
        with WaitCursor():
            save_file()

            file_path = get_file_path()

            self.report({'INFO'}, file_path)
            self.report({'INFO'}, get_db_path(file_path))

            message = str(context.scene.checkpoint_message)
            context.scene.checkpoint_message = ""

            result = call_lib({
                'command': 'create-checkpoint',
                'db_path': get_db_path(file_path),
                'path_to_blend': file_path,
                'message': message
            })

            if 'error' in result:
                self.report({'ERROR'}, str(result))
                return {'FINISHED'}

            # checkpoints
            global CURRENT_CHECKPOINT_HASH
            CURRENT_CHECKPOINT_HASH = result['current_checkpoint_hash']
            global CHECKPOINT_ITEMS
            CHECKPOINT_ITEMS = result['checkpoints_on_this_branch']

            set_branches_checkpoints_from_state(context)

            return {'FINISHED'}


class CheckpointsList(bpy.types.UIList):
    """List of Checkpoints"""
    layout_type = "DEFAULT"
    bl_idname = "CheckpointsList"

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row()

        global CURRENT_CHECKPOINT_HASH
        icon = CHECKMARK_ICON if item.hash == CURRENT_CHECKPOINT_HASH else BLANK_ICON
        row.label(text=item.message, icon=icon)
        restore_op = row.operator(RestoreOperator.bl_idname,
                                  text="Restore")
        restore_op.hash = item.hash

        restore_to_file_op = row.operator(
            RestoreToFileOperator.bl_idname, text="", icon=TRIA_RIGHT)
        restore_to_file_op.hash = item.hash
        restore_to_file_op.name = item.message


class BranchesList(bpy.types.UIList):
    """List of Branches"""
    layout_type = "DEFAULT"
    bl_idname = "BranchesList"

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row()

        global CURRENT_BRANCH
        icon = CHECKMARK_ICON if item.name == CURRENT_BRANCH else BLANK_ICON
        row.label(text=item.name, icon=icon)
        row.operator(SwitchBranchesOperator.bl_idname,
                     text="Switch to").name = item.name


class TimelinePanel(bpy.types.Panel):
    bl_label = "Timeline"
    bl_idname = "TimelinePanel"
    bl_space_type = 'PROPERTIES'
    bl_region_type = 'WINDOW'
    bl_context = "world"
    bl_category = 'Timeline'
    bl_options = {'HEADER_LAYOUT_EXPAND'}

    def draw(self, context):
        layout = self.layout

        global CONNECTED
        if not CONNECTED:
            layout.operator(ConnectOperator.bl_idname,
                            text="Connect to Timeline")
            layout.operator(BlendFileFromTimelineOperator.bl_idname,
                            text="Generate .blend file from Timeline")
            return

        # BRANCHES
        branches_box = layout.box()
        branches_box.label(text="Branches")

        global CURRENT_BRANCH
        current_branch_row = branches_box.row()
        current_branch_row.label(text="Current branch: ")
        current_branch_row.label(text=CURRENT_BRANCH)

        branches_box.template_list(BranchesList.bl_idname, "Branches",
                                   context.scene, "branch_items", context.scene, "branch_idx", rows=5)
        branches_box.operator("wm.new_branch_operator", text="New branch")

        # CHECKPOINT ITEMS
        restore_box = layout.box()
        restore_box.label(text="Restore checkpoint")
        restore_box.template_list(CheckpointsList.bl_idname, "Restore Checkpoint",
                                  context.scene, "checkpoint_items", context.scene, "checkpoint_idx", rows=5)

        # NEW CHECKPOINT
        checkpoint_box = layout.box()
        checkpoint_box.label(text="New Checkpoint")
        checkpoint_box.prop(context.scene, "checkpoint_message", text="")
        checkpoint_box.operator(
            "my.create_checkpoint_operator", text="Create Checkpoint")

        layout.operator(ConnectOperator.bl_idname,
                        text="Reconnect")


def register():
    bpy.utils.register_class(ConnectOperator)
    bpy.utils.register_class(BlendFileFromTimelineOperator)
    bpy.utils.register_class(SwitchBranchesOperator)
    bpy.utils.register_class(NewBranchOperator)
    bpy.utils.register_class(CheckpointItem)
    bpy.utils.register_class(BranchItem)
    bpy.utils.register_class(RestoreOperator)
    bpy.utils.register_class(RestoreToFileOperator)
    bpy.utils.register_class(CreateCheckpointOperator)
    bpy.utils.register_class(CheckpointsList)
    bpy.utils.register_class(BranchesList)
    bpy.utils.register_class(TimelinePanel)

    # placeholders for the template list
    bpy.types.Scene.checkpoint_idx = bpy.props.IntProperty()
    bpy.types.Scene.branch_idx = bpy.props.IntProperty()

    # items for the template list
    bpy.types.Scene.checkpoint_items = bpy.props.CollectionProperty(
        type=CheckpointItem)
    bpy.types.Scene.branch_items = bpy.props.CollectionProperty(
        type=BranchItem)

    bpy.types.Scene.checkpoint_message = bpy.props.StringProperty(
        name="", options={'TEXTEDIT_UPDATE'})


def unregister():
    global LOADED_LIB_CACHE
    if LOADED_LIB_CACHE != None:
        unload_lib(LOADED_LIB_CACHE)
    del LOADED_LIB_CACHE

    bpy.utils.unregister_class(ConnectOperator)
    bpy.utils.unregister_class(BlendFileFromTimelineOperator)
    bpy.utils.unregister_class(SwitchBranchesOperator)
    bpy.utils.unregister_class(NewBranchOperator)
    bpy.utils.unregister_class(CheckpointItem)
    bpy.utils.unregister_class(BranchItem)
    bpy.utils.unregister_class(RestoreOperator)
    bpy.utils.unregister_class(RestoreToFileOperator)
    bpy.utils.unregister_class(CreateCheckpointOperator)
    bpy.utils.unregister_class(CheckpointsList)
    bpy.utils.unregister_class(BranchesList)
    bpy.utils.unregister_class(TimelinePanel)

    del bpy.types.Scene.checkpoint_idx
    del bpy.types.Scene.branch_idx
    del bpy.types.Scene.checkpoint_items
    del bpy.types.Scene.branch_items
    del bpy.types.Scene.checkpoint_message


if __name__ == "__main__":
    register()

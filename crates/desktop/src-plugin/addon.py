import bpy
import requests
import os
import json
from threading import Timer
from urllib.parse import quote_plus

bl_info = {
    "name": "Timeline Tool",
    "blender": (2, 80, 0),
    "category": "Object",
}

URL = 'http://127.0.0.1:8080'

API_NOT_AVAILABLE_ERROR = "Server not running!"
CANNOT_CONNECT_ERROR = "Cannot connect to timeline!"


CHECKMARK_ICON = 'CHECKMARK'
BLANK_ICON = 'BLANK1'


def get_file_path():
    return bpy.data.filepath


def get_db_path(file_path):
    filename = os.path.basename(file_path)
    filename_with_ext = '.' + filename + '.timeline'
    return os.path.join(os.path.dirname(file_path), filename_with_ext)


def call_healthcheck_api():
    try:
        requests.get(f"{URL}/healthcheck").raise_for_status()
        return True
    except requests.exceptions.RequestException:
        return None


def with_healthcheck(function):
    def wrapper(*args, **kwargs):
        running = call_healthcheck_api()
        if not running:
            return (False, API_NOT_AVAILABLE_ERROR)
        return function(*args, **kwargs)
    return wrapper


@with_healthcheck
def call_connect_api():
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/connect'
    headers = {'Content-Type': 'application/json'}
    data = {
        'db_path': db_path,
        'file_path': file_path,
    }

    try:
        requests.post(url, headers=headers,
                      data=json.dumps(data)).raise_for_status()
        return (True, None)
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_commit_api(message):
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/commit'
    headers = {'Content-Type': 'application/json'}
    data = {
        'db_path': db_path,
        'file_path': file_path,
        'message': message,
    }

    try:
        requests.post(url, headers=headers,
                      data=json.dumps(data)).raise_for_status()
        return (True, None)
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_checkpoints_api(current_branch):
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/checkpoints/{quote_plus(db_path)}/{quote_plus(current_branch)}'

    try:
        response = requests.get(url)
        response.raise_for_status()
        return (True, response.json())
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_restore_api(hash):
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/restore'
    headers = {'Content-Type': 'application/json'}
    data = {
        'db_path': db_path,
        'file_path': file_path,
        'hash': hash,
    }

    try:
        requests.post(url, headers=headers,
                      data=json.dumps(data)).raise_for_status()
        return (True, None)
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_new_branch_api(new_branch_name):
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/branches/new'
    headers = {'Content-Type': 'application/json'}
    data = {
        'db_path': db_path,
        'file_path': file_path,
        'branch_name': new_branch_name,
    }

    try:
        requests.post(url, headers=headers,
                      data=json.dumps(data)).raise_for_status()
        return (True, None)
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_list_branches_api():
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/branches/{quote_plus(db_path)}'

    try:
        response = requests.get(url)
        response.raise_for_status()
        return (True, response.json())
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_get_current_branch_api():
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/branches/current/{quote_plus(db_path)}'

    try:
        response = requests.get(url)
        response.raise_for_status()
        return (True, response.json())
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_get_current_commit_hash():
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/commit/current/{quote_plus(db_path)}'

    try:
        response = requests.get(url)
        response.raise_for_status()
        return (True, response.json())
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


@with_healthcheck
def call_switch_branch_api(new_branch_name):
    file_path = get_file_path()
    db_path = get_db_path(file_path)
    url = f'{URL}/branches/switch'
    headers = {'Content-Type': 'application/json'}
    data = {
        'db_path': db_path,
        'file_path': file_path,
        'branch_name': new_branch_name,
    }

    try:
        requests.post(url, headers=headers,
                      data=json.dumps(data)).raise_for_status()
        return (True, None)
    except requests.exceptions.RequestException as err:
        return (False, err.response.json())


def call_list_branches_operator():
    bpy.ops.wm.list_branches_operator()


def call_list_checkpoints_operator():
    bpy.ops.wm.list_checkpoints_operator()


def call_get_current_branch_operator():
    bpy.ops.wm.get_current_branch_operator()


def call_call_get_current_commit_hash_operator():
    bpy.ops.wm.get_current_commit_hash_operator()


def call_connect_operator():
    bpy.ops.wm.connect()


def run_onload_ops():
    call_get_current_branch_operator()
    call_list_branches_operator()
    call_list_checkpoints_operator()
    call_call_get_current_commit_hash_operator()


def save_file():
    bpy.ops.file.pack_all()
    bpy.ops.wm.save_mainfile()


def refresh_file():
    bpy.ops.wm.revert_mainfile()


class WaitCursor():
    def __enter__(self):
        bpy.context.window.cursor_set("WAIT")

    def __exit__(self, *args, **kwargs):
        pass


class ConnectOperator(bpy.types.Operator):
    """Connect to the Timeline"""
    bl_idname = "wm.connect"
    bl_label = "Connect to the Timeline"

    def execute(self, context):
        (success, _) = call_connect_api()
        if not success:
            self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
            return {'FINISHED'}
        bpy.types.Scene.connected = True
        run_onload_ops()

        return {'FINISHED'}


class ListCheckpointsOperator(bpy.types.Operator):
    """List checkpoints for this branch"""
    bl_idname = "wm.list_checkpoints_operator"
    bl_label = "List Checkpoints"

    def execute(self, context):
        current_branch = context.scene.current_branch if context.scene.current_branch != None else "main"
        (success, result) = call_checkpoints_api(current_branch)
        if not success and result == API_NOT_AVAILABLE_ERROR:
            self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
            return {'FINISHED'}

        if not success:
            self.report({'ERROR'}, f"Cannot list checkpoints: {result}")
            return {'FINISHED'}

        bpy.context.scene.checkpoint_items.clear()

        for i in result:
            item = bpy.context.scene.checkpoint_items.add()
            item.hash = i['hash']
            item.message = i['message']

        return {'FINISHED'}


class ListBranchesOperator(bpy.types.Operator):
    """List branches for this project"""
    bl_idname = "wm.list_branches_operator"
    bl_label = "List Branches"

    def execute(self, context):
        (success, response) = call_list_branches_api()
        if not success and response == API_NOT_AVAILABLE_ERROR:
            self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
            return {'FINISHED'}

        if not success:
            self.report({'ERROR'}, f"Cannot list branches: {response}")
            return {'FINISHED'}

        bpy.context.scene.branch_items.clear()

        for branch in response:
            item = bpy.context.scene.branch_items.add()
            item.name = branch

        return {'FINISHED'}


class GetCurrentBranchOperator(bpy.types.Operator):
    """Find out which branch is active"""
    bl_idname = "wm.get_current_branch_operator"
    bl_label = "Get current branch"

    def execute(self, context):
        (success, response) = call_get_current_branch_api()
        if not success and response == API_NOT_AVAILABLE_ERROR:
            self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
            return {'FINISHED'}

        if not success:
            self.report({'ERROR'}, f"Cannot get current branch: {response}")
            return {'FINISHED'}

        context.scene.current_branch = response

        return {'FINISHED'}


class GetCurrentCommitHashOperator(bpy.types.Operator):
    """Get the hash of the current commit"""
    bl_idname = "wm.get_current_commit_hash_operator"
    bl_label = "Get the hash of the current commit"

    def execute(self, context):
        (success, response) = call_get_current_commit_hash()
        if not success and response == API_NOT_AVAILABLE_ERROR:
            self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
            return {'FINISHED'}

        if not success:
            self.report(
                {'ERROR'}, f"Cannot get latest commit hash: {response}")
            return {'FINISHED'}

        context.scene.current_commit_hash = response

        return {'FINISHED'}


class SwitchBranchesOperator(bpy.types.Operator):
    """Switch to another branch"""
    bl_idname = "wm.switch_branch_operator"
    bl_label = "Switch Branch"

    name: bpy.props.StringProperty(name="Branch name", default="")

    def execute(self, _):
        with WaitCursor():
            # TODO: if the file is unsaved, ask the user to confirm
            save_file()
            (success, response) = call_switch_branch_api(self.name)
            if not success and response == API_NOT_AVAILABLE_ERROR:
                self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
                return {'FINISHED'}

            if not success:
                self.report({'ERROR'}, f"Cannot switch branch: {response}")
                return {'FINISHED'}

            refresh_file()
            run_onload_ops()
            return {'FINISHED'}


class NewBranchOperator(bpy.types.Operator):
    """Create a new branch"""
    bl_idname = "wm.new_branch_operator"
    bl_label = "New Branch"

    name: bpy.props.StringProperty(name="Branch name", default="")

    def execute(self, _):
        with WaitCursor():
            (success, response) = call_new_branch_api(self.name)
            if not success and response == API_NOT_AVAILABLE_ERROR:
                self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
                return {'FINISHED'}

            if not success:
                self.report({'ERROR'}, f"Cannot create new branch: {response}")
                return {'FINISHED'}

            run_onload_ops()
            return {'FINISHED'}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


class CheckpointItem(bpy.types.PropertyGroup):
    message: bpy.props.StringProperty(name="Message", default="")
    hash: bpy.props.StringProperty(name="Hash", default="")


class BranchItem(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(name="Name", default="")


class RestoreOperator(bpy.types.Operator):
    """Restore a checkpoint"""
    bl_idname = "my.restore_operator"
    bl_label = "Restore"

    hash: bpy.props.StringProperty(name="Hash", default="")

    def execute(self, context):
        with WaitCursor():
            # TODO: if the file is unsaved, ask the user to confirm
            (success, response) = call_restore_api(self.hash)
            if not success and response == API_NOT_AVAILABLE_ERROR:
                self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
                return {'FINISHED'}

            if not success:
                self.report(
                    {'ERROR'}, f"Cannot restore checkpoint: {response}")
                return {'FINISHED'}

            refresh_file()
            run_onload_ops()
            return {'FINISHED'}


class CreateCheckpointOperator(bpy.types.Operator):
    """Create a new checkpoint"""
    bl_idname = "my.create_checkpoint_operator"
    bl_label = "Create Checkpoint Operator"

    def execute(self, context):
        with WaitCursor():
            save_file()

            message = context.scene.commit_message
            context.scene.commit_message = ""
            (success, response) = call_commit_api(message)
            if not success and response == API_NOT_AVAILABLE_ERROR:
                self.report({'ERROR'}, API_NOT_AVAILABLE_ERROR)
                return {'FINISHED'}

            if not success:
                self.report({'ERROR'}, f"Cannot create checkpoint: {response}")
                return {'FINISHED'}

            run_onload_ops()
            return {'FINISHED'}


class RefreshOperator(bpy.types.Operator):
    """Refresh the plugin UI"""
    bl_idname = "my.refresh"
    bl_label = "Refresh"

    def execute(self, _):
        run_onload_ops()
        return {'FINISHED'}


class CheckpointsList(bpy.types.UIList):
    """List of Checkpoints"""
    layout_type = "DEFAULT"
    bl_idname = "CheckpointsList"

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row()
        icon = CHECKMARK_ICON if item.hash == context.scene.current_commit_hash else BLANK_ICON
        row.label(text=item.message, icon=icon)
        row.operator("my.restore_operator",
                     text="Restore").hash = item.hash


class BranchesList(bpy.types.UIList):
    """List of Branches"""
    layout_type = "DEFAULT"
    bl_idname = "BranchesList"

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row()
        icon = CHECKMARK_ICON if item.name == context.scene.current_branch else BLANK_ICON
        row.label(text=item.name, icon=icon)
        row.operator("wm.switch_branch_operator",
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

        if not context.scene.connected:
            layout.operator("wm.connect", text="Connect to Timeline")
            return

        # BRANCHES
        branches_box = layout.box()
        branches_box.label(text="Branches")

        current_branch_row = branches_box.row()
        current_branch_row.label(text="Current branch: ")
        current_branch_row.label(text=context.scene.current_branch)

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
        checkpoint_box.prop(context.scene, "commit_message", text="")
        checkpoint_box.operator(
            "my.create_checkpoint_operator", text="Create Checkpoint")

        layout.operator("my.refresh", text="Refresh")


def register():
    bpy.utils.register_class(ConnectOperator)
    bpy.utils.register_class(ListCheckpointsOperator)
    bpy.utils.register_class(ListBranchesOperator)
    bpy.utils.register_class(SwitchBranchesOperator)
    bpy.utils.register_class(NewBranchOperator)
    bpy.utils.register_class(GetCurrentBranchOperator)
    bpy.utils.register_class(GetCurrentCommitHashOperator)
    bpy.utils.register_class(CheckpointItem)
    bpy.utils.register_class(BranchItem)
    bpy.utils.register_class(RestoreOperator)
    bpy.utils.register_class(CreateCheckpointOperator)
    bpy.utils.register_class(RefreshOperator)
    bpy.utils.register_class(CheckpointsList)
    bpy.utils.register_class(BranchesList)
    bpy.utils.register_class(TimelinePanel)

    bpy.types.Scene.checkpoint_idx = bpy.props.IntProperty()
    bpy.types.Scene.branch_idx = bpy.props.IntProperty()
    bpy.types.Scene.connected = bpy.props.BoolProperty(
        name='connected', default=False)
    bpy.types.Scene.checkpoint_items = bpy.props.CollectionProperty(
        type=CheckpointItem)
    bpy.types.Scene.branch_items = bpy.props.CollectionProperty(
        type=BranchItem)
    bpy.types.Scene.current_branch = bpy.props.StringProperty(name="")
    bpy.types.Scene.current_commit_hash = bpy.props.StringProperty(name="")
    bpy.types.Scene.commit_message = bpy.props.StringProperty(
        name="", options={'TEXTEDIT_UPDATE'})

    Timer(100, run_onload_ops, ()).start()


def unregister():
    bpy.utils.unregister_class(ConnectOperator)
    bpy.utils.unregister_class(ListCheckpointsOperator)
    bpy.utils.unregister_class(ListBranchesOperator)
    bpy.utils.unregister_class(SwitchBranchesOperator)
    bpy.utils.unregister_class(NewBranchOperator)
    bpy.utils.unregister_class(GetCurrentBranchOperator)
    bpy.utils.unregister_class(GetCurrentCommitHashOperator)
    bpy.utils.unregister_class(CheckpointItem)
    bpy.utils.unregister_class(BranchItem)
    bpy.utils.unregister_class(RestoreOperator)
    bpy.utils.unregister_class(CreateCheckpointOperator)
    bpy.utils.unregister_class(RefreshOperator)
    bpy.utils.unregister_class(CheckpointsList)
    bpy.utils.unregister_class(BranchesList)
    bpy.utils.unregister_class(TimelinePanel)

    del bpy.types.Scene.checkpoint_idx
    del bpy.types.Scene.branch_idx
    del bpy.types.Scene.connected
    del bpy.types.Scene.checkpoint_items
    del bpy.types.Scene.branch_items
    del bpy.types.Scene.commit_message
    del bpy.types.Scene.current_branch
    del bpy.types.Scene.current_commit_hash


if __name__ == "__main__":
    register()

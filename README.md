![main](https://github.com/bkrmendy/cg_timeline/actions/workflows/rust.yml/badge.svg)

# Timeline (beta)

## TLDR

Timeline is a Blender addon for maintaining multiple versions of the same
project. These can be either parallel versions ("branches") or the versions can
build on each other ("checkpoints").

In other words, Timeline is a very basic version control tool tailored to
Blender.

<img width="1239" alt="image" src="https://github.com/bkrmendy/cg_timeline/assets/16385508/d44d6dd2-68b2-4ee2-973f-b99e3624d970">

[Demo video](https://youtu.be/acJvUIIRbJc)


## Installation

Download the latest release for your operating system/process architecture from
Releases. Then, install the addon .zip file from the Preferences panel of
Blender by clicking `Install` and then choosing the downloaded addon.

> As of now, only Intel processors are supported. Support for more platforms
> will be added in the future. Alternatively, you can build the addon from
> source if you have the Rust toolchain installed.

## Overview

Timelines are built on two main concepts: checkpoints and branches.

A checkpoint is a snapshot of the Blend file at a given point in time.

A branch is a list of checkpoints, ordered from oldest to newest. Each timeline
has a `main` branch, which cannot be deleted.

## Connecting to the Timeline

When you open a Blend file, you have to connect to the Timeline. To connect,
click `Connect to Timeline` in the Timeline section. This will either create a
new timeline DB for the Blender project or connect to an existing one. The name
of the timeline DB for a blend file is the same as the name of the blend file
with `.timeline` appended (ie., if you have a file called `character.blend`, the
timeline DB will be called `character.blend.timeline`).

> If you rename a Blend file that has a timeline DB, you have to rename the
> timeline DB too! Otherwise, the timeline addon will create a brand-new
> timeline DB, instead of using the original one.

You can also generate a Blend file from a timeline DB. To do this, click
`Generate .blend file from Timeline` (below `Connect to Timeline`). This will
create a new Blend file from the latest checkpoint of the `main` branch in the
chosen timeline DB, and load that file in Blender.

## Creating checkpoints

You can create checkpoints by typing a checkpoint name into the box in the
`New checkpoint` section, and clicking `Create checkpoint`. Timeline will create a
snapshot of the whole blend file, and will store it in the Timeline DB.

## Restoring checkpoints

You can restore a given checkpoint by clicking the `Restore` button next to a
checkpoint's name. Restoring a checkpoint erases any unsaved changes.

You can also export the contents of a checkpoint into a new blend file by
clicking the arrow next to the `Restore` button. The new blend file will show up
next to the open blend file, and its name will be the same as the name of the
checkpoint.

## Creating branches

A branch is a succession of checkpoints building on each other. Branches make it
possible to create multiple coexisting versions of a file (such as multiple
variations of the same model).

You can create a new branch by clicking the `New branch` button in the
`Branches` section, typing in a name for the branch, and clicking `OK`. Then,
the new branch will become active.

## Switching branches

You can switch between branches by clicking on the `Switch to` button next to the
name of the branch. When switching to a new branch, the blend file will be
updated with the contents of the most recent checkpoint on that branch.

## Deleting branches

If you're 100% certain that you won't need a branch anymore, you can delete it
by clicking the `X` button next to the `Switch to` button. This will delete that
branch, and all associated checkpoints.

> This feature might be tweaked in the future

# Timeline (beta)

> TODO: demo gif

## TLDR

Timeline is a Blender addon for maintaining multiple versions of the same
project. These can be either parallel versions ("branches") or the versions can
build on each other ("checkpoints").

In other words, Timeline is a very basic version control tool tailored to
Blender.

## Table of Contents

> TODO

## Installation

> TODO

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

> TODO

## Restoring checkpoints

> TODO

## Creating branches

> TODO

## Switching branches

> TODO

## Deleting branches

> TODO

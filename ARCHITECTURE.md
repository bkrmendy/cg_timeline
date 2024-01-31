# Architecture

## Table of contents

1. [Overview](#Overview)
2. [The Blender add-on](#the-blender-add-on)
3. [The Rust library](#the-rust-library)
4. [Resources](#resources)

## Overview

Timeline has two main components: the Blender addon and the Rust library, with
the Blender addon providing a frontend for the Rust library. The Rust library is
compiled into a dylib that the addon loads at runtime. The add-on communicates
with the Rust ib through the `call_command` function, which expects a JSON
string with the command data.

The advantages of this setup are:

- the addon easy to package and distribute
- the boilerplate for calling functions from the Rust lib is minimal and very
  flexible, it's almost like the Rust library was a REST API.

A drawback is that the Rust lib is a black box to the Python addon, which makes
debugging harder (the two components have to be debugged separately).

## The Blender add-on

The Blender add-on code is in `timeline/__init.py__`. It's a standard Blender
addon that loads `libtimeline.dylib`, and calls it via the `call_command`
function exposed in the dylib.

## The Rust library

The library implements the bulk of the addon's functionality. It's built on two
main building blocks: the Blend file parsing code (in `src/blend/`) and the
SQLite schema (defined in `src/db/db_ops.rs`).

### Command architecture

Commands are the capabilities the Rust library exposes to the Blender addon.
When a command is called from the Blender addon, it goes through the following
layers:

- `do_command` in `src/ffi.rs` decodes the JSON payload
- A command handler interprets the command. These handlers are composed of
  helper functions from the files in `src/api/`, which provide a high-level API
  for interacting with the underlying SQLite DB. These helpers from `src/api/`
  either call each other or use `Persistence` to interact with the SQLite DB.
- `Persistence` (defined in `src/db_ops`) provides a low-level API for
  interacting with the SQLite DB. It's mostly oriented towards
  reading/writing/checking the existence of a single record in one of the
  tables.

### DB Schema

#### Block storage

Timeline stores Blend files in a deduplicated way, so only the changes between
checkpoints are saved, and the unchanged bits are shared between checkpoints.
The way this is achieved is that the Blend file is parsed into its constituent
file blocks, the file blocks are hashed, and stored with the block hash acting
as the key and the blob of the block acting as the value (see the `blocks` table
in `src/db/db_ops.rs`).

#### Checkpoints

The schema for checkpoints is defined in the `commits` table in
`src/db/db_ops.rs`. A checkpoint is essentially the analog of a Git commit.

> TODO: rename `commits` to `checkpoints`

The most important part of the checkpoint metadata is `blocks_and_pointers`,
which stores the hashes of the blocks making up the blend file corresponding to
that checkpoint, with any pointers in those blocks.

Blocks are deduplicated by hashing their contents. However, the blocks store a
dump of the raw data used by Blender, which might include pointers. The values
of these pointers are actual memory locations, not pointers to some offset
within the Blend file, and they can change every time the Blend file is opened.
To prevent these pointers from breaking the content-based addressing, when the
Blend file is parsed, they are replaced with null pointers in the parsed struct,
and their original values are returned alongside the parsed, "fixed-up" Blend
file.

Checkpoints also include the hash of the Blend file they were created from,
which acts as the unique ID of the checkpoint. This hash is used to find the
ancestors of a checkpoint.

### Blend file parsing

The Blend file parser is implemented as a simple recursive-descent parser.
First, the file blocks are parsed from the Blend file, and then the individual
blocks are parsed, so that any pointers in the blocks can be noted down and then
erased. Java .Blend was used as a reference for block parsing.

## Resources

[Java .Blend](https://www.blendernation.com/2016/01/05/java-blend/)

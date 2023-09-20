```
 _____ _____ _____ ________  ___ _____ _     _____ _   _  _____
/  __ \  __ \_   _|_   _|  \/  ||  ___| |   |_   _| \ | ||  ___|
| /  \/ |  \/ | |   | | | .  . || |__ | |     | | |  \| || |__
| |   | | __  | |   | | | |\/| ||  __|| |     | | | . ` ||  __|
| \__/\ |_\ \ | |  _| |_| |  | || |___| |_____| |_| |\  || |___
 \____/\____/ \_/  \___/\_|  |_/\____/\_____/\___/\_| \_/\____/

```

## Architecture

### Summary

The project is structured as a cargo workspace with three crates:

- `timeline_lib`: Named as it is for historical reasons, it contains the code that implements the actual parser-printer, reading and writing blend files, and code that interacts with the timeline DB. These could be broken up further, with the parser-printer code completely separated (since its interface is already pretty stable).
- `cli` is a command-line interface to the functionality implemented in the `timeline_lib` crate. It's mainly useful to debug specific commands and illustrate how they can be used.
- `desktop` implements the primary way of interacting with the tool: A server wrapped in a Tauri app, which exposes a REST API called by a Blender addon (`addon.py`).

### `Parserprinter`

The `Parserprinter` crate contains the following modules:

#### `printer_parser`

The "original" printer-parser library (`TODO`: more on this)

#### `blend`

Implements a collection of functions related to reading/writing `.blend` files, heavily relying on the functionality implemented on `printer_parser`. Currently `blend` doesn't parse the contents of file blocks, though `SimpleParsedBlock` contains a lot of info that can be used as a jumping-off point for more in-depth block processing.

#### `db`

API for the timeline DB. The timeline DB is actually a folder that contains two separate DBs:

- A SQLite DB to store commits
- A RockDB DB to store blobs and key/value config

The tradeoff here was that the writing blocks into SQLite was super slow, on the other hand, the same was really fast in RocksDB. It's entirely possible that SQLite is being misused when writing key-value data sequentially, so there might be a lot of improvement here. The SQLite DB was kept so that list/filter type queries can be implemented efficiently (which wouldn't be trivial/elegant in RocksDB).

#### `api`

`api` implements a collection of high-level operations, each of which composes multiple DB API calls.
These operations aim to preserve invariants before/after DB operations.

#### Wire format

The API features a very basic export/import functionality, which can export/import a number of commits and the blocks they refer to. This is a binary format, implemented with the `printer_parser` machinery (see `exchange.rs` for details).

Since the format only includes commits and blocks, the branches they refer to have to be reconstructed from the commits. Also, it's not guaranteed that the exported commits are actually rooted in `main` (but this is a big blind spot in the rest of the code too). On the other hand, this format makes it dead simple to sync to other timeline DBs, since the transmitted commits describe themselves.

### `cli`

`cli` uses `clap` to implement a thin wrapper around the high-level ops in `api`.

### `desktop`

`desktop` implement the real "frontend" of the tool. It's composed of two main components:

- A Tauri app, which runs a server that exposes a REST API.
- A Blender addon that communicates with the Tauri app through that API.

The reason for this seemingly cumbersome setup is that it's not trivial to package/distribute a Rust tool as a blender addon (since it's not possible have a one-click installer that installs both the Rust tool and the addon).

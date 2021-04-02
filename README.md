# Kasten

Kasten is a file hosting server that allows uploading, accessing and sharing of files with the browser. It is mainly written in Rust using [Rocket](rocket.rs).

The project also contains the tool `bonbon`, which allows for adding, listing and removing users.

## Prerequisites

Kasten was written to run on GNU/Linux. While it may work on other systems, it was only tested on Arch Linux and Ubuntu 18.04.

## Installing Kasten

To build Kasten, follow these steps:

```
cargo build --release
```

To build `bonbon`, follow these steps:

```
cd bonbon
cargo build --release
```

You can compile both binaries by running the following command from the projects root:

```
cargo build --release --workspace
```

## Using Kasten

To use Kasten, just execute `./target/release/kasten`.
Kasten supports the following environment variables:
- KASTEN_DB_LOCATION: The path at which the database is saved. (Defaults to ./var/server-sled-db)
- KASTEN_FILE_LOCATION: The path at which the uploaded files are saved. (Defaults to ./var/files)


To use `bonbon` run `./target/release/bonbon [command]`.
To get a list of available commands run `./target/release/bonbon --help`.

## TODO
- File encryption
- Removing files/directories
- Adding access rights


## Contact

If you want to contact me you can reach me at paul.pumpernickel@posteo.eu.

## License

This project uses the following license: [GNU GPLv3](www.gnu.org/licenses/gpl-3.0.html).

# UNIT3D-Announce

High-performance backend BitTorrent tracker compatible with UNIT3D tracker software.

## Usage

```sh
# Clone this repository
$ git clone https://github.com/HDInnovations/UNIT3D-Announce

# Go into the repository
$ cd UNIT3D-Announce

# Rename .env.example to .env
$ mv .env.example .env

# Adjust configuration as necessary
$ nano .env

# Build the tracker
$ cargo build --release

# Run the tracker
$ target/release/unit3d-announce
```

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

## Reverse proxy

If you serve both UNIT3D and UNIT3D-Announce on the same domain, add the following `location` block to your nginx configuration already used for UNIT3D:

```nginx
     location /announce/ {
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header Host $host;
        proxy_pass http://aaa.bbb.ccc.ddd:eeee$request_uri;
        real_ip_recursive on;
        set_real_ip_from fff.ggg.hhh.iii;
    }
```

- `aaa.bbb.ccc.ddd:eeee` is the IP address and port of UNIT3D-Announce. By default, UNIT3D-Announce is served on `0.0.0.0:3001`, but it will be configurable via environment variables in the future. Replace this value with your true listening socket.
- `fff.ggg.hhh.iii` is the IP address of the nginx proxy. You can add additional `set_real_ip_from jjj.kkk.lll.mmm;` lines for each additional proxy used so long as the proxy appends the proper values to the `X-Forwarded-For` header. Replace this with your proxy IP address.

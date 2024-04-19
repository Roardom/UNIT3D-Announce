# UNIT3D-Announce

High-performance backend BitTorrent tracker compatible with UNIT3D tracker software.

⚠️ **This is unstable, experimental code and should not be used in production** ⚠️

## Usage

```sh
# Go to where UNIT3D is already installed
$ cd /var/www/html

# Clone this repository
$ git clone -b main https://github.com/HDInnovations/UNIT3D-Announce unit3d-announce

# Go into the repository
$ cd unit3d-announce

# Copy .env.example to .env
$ cp .env.example .env

# Adjust configuration as necessary
$ nano .env

# Build the tracker
$ cargo build --release

# Go into UNIT3D's base directory
$ cd /var/www/html

# Add the required environment variables to UNIT3D'S .env file:
# (`TRACKER_HOST`, `TRACKER_PORT`, and `TRACKER_KEY`)
# These values should match their respective values in UNIT3D-Announce's .env file:
# (`LISTENING_IP_ADDRESS`, `LISTENING_PORT`, and `APIKEY`)
$ nano .env

# Enable the external tracker in UNIT3D's config
$ nano config/announce.php

# Go back into UNIT3D-Announce's directory
$ cd unit3d-announce

# Run the tracker
$ target/release/unit3d-announce
```

## Reverse proxy

If you serve both UNIT3D and UNIT3D-Announce on the same domain, add the following `location` block to your nginx configuration already used for UNIT3D.

```sh
# Edit nginx config
$ nano /etc/nginx/sites-enabled/default
```

Paste the following `location` block into the first `server` block immediately after the last existing `location` block.

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

- `aaa.bbb.ccc.ddd:eeee` is the local listening IP address and port of UNIT3D-Announce. Set this to the `LISTENING_IP_ADDRESS` and `LISTENING_PORT` configured in the .env file.
- `fff.ggg.hhh.iii` is the public listening IP address of the nginx proxy used for accessing the frontend website. You can add additional `set_real_ip_from jjj.kkk.lll.mmm;` lines for each additional proxy used so long as the proxy appends the proper values to the `X-Forwarded-For` header. Replace this with your proxy IP address.


```sh
# Reload nginx once finished
$ service nginx reload
```

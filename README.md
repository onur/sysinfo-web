# sysinfo-web

[![Build Status](https://secure.travis-ci.org/onur/sysinfo-web.svg?branch=master)](https://travis-ci.org/onur/sysinfo-web)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/onur/sysinfo-web/master/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/sysinfo-web.svg)](https://crates.io/crates/sysinfo-web)

Lightweight web based process viewer built on top of 
[sysinfo](https://github.com/GuillaumeGomez/sysinfo).

[See a demo of sysinfo-web](https://docs.rs/@sysinfo/).

## Installation and usage

You can grab a precompiled binary from
[releases](https://github.com/onur/sysinfo-web/releases) page or you can install
sysinfo-web with cargo:

```sh
cargo install --git https://github.com/onur/sysinfo-web
```

Make sure `sysinfo-web` is in your `PATH` and you can run it with:

```
sysinfo-web <SOCKADDR>
```

Socket address is optional, by default it will listen: <http://localhost:3000/>.


## Screenshot

[![sysinfo-web](https://i.imgur.com/qQPe9yN.png)](https://i.imgur.com/RH8l8Sz.png)

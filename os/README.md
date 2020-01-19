<h1 align="center">A User Space Operating System for Data Analytics</h1>

Arcon OS aims to provide a lightweight self-contained system for data analytics
that may be deployed on a range of hardware. It utilises Linux namespaces to isolate itself 
from the actual OS and cgroups to manage relevant resources (e.g., memory & cpu) within the instance.

## Overview

TODO

## Building

The project is currently only developed using x86_64-unknown-linux-(gnu/musl). 
The following script will build a static musl binary in release mode.

```
$ ./bin/build-musl.sh
```

Whereas if you are just developing, simply run:

```
$ cargo build
```

## Generating a rootfs

The project uses the Alpine filesystem (~5MB). The following script will download and extract an Alpine x86_64 rootfs:

```
$ ./bin/image.sh
```

The [`images`] directory contains more image options, but requires docker to build and export. 
The [`x86_64_netdata`] image includes the [netdata](https://github.com/netdata/netdata) dashboard 
that provides real-time monitoring of the Dragonslayer instance. However, this increases the required disk space.


[`images`]: images
[`x86_64_netdata`]: images/x86_64_netdata

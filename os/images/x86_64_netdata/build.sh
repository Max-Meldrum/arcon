#!/bin/bash

mkdir -p rootfs
docker build -t arconos_x86_64_netdata .
docker export $(docker create arconos_x86_64_netdata) | tar -C rootfs -xf -

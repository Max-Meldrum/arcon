#!/bin/bash

mkdir -p rootfs
docker build -t arconos_x86_64.
docker export $(docker create arconos_x86_64) | tar -C rootfs -xf -

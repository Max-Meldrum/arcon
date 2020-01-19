#!/bin/bash

# Can be used to download a x86_64 rootfs without docker installed

ALPINE_BASE_VER="3.10"
ALPINE_VER="3.10.3"
ALPINE_TAR_NAME="alpine-minirootfs-3.10.3-x86_64.tar.gz"
ALPINE_URL="http://dl-cdn.alpinelinux.org/alpine/v$ALPINE_BASE_VER/releases/x86_64/$ALPINE_TAR_NAME"

if [ -f "$ALPINE_TAR_NAME" ]
then
    rm -rf rootfs
    mkdir rootfs
    tar -xf $ALPINE_TAR_NAME -C rootfs/
else
    wget $ALPINE_URL
    rm -rf rootfs
    mkdir rootfs

    tar -xf $ALPINE_TAR_NAME -C rootfs/
fi

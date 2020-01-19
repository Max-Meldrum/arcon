FROM multiarch/alpine:amd64-edge

MAINTAINER Max Meldrum <mmeldrum@kth.se>

# The part below is taken from https://github.com/woahbase/alpine-netdata
ARG PUID=1000
ARG PGID=1000
#
RUN set -xe \
    && apk add -Uu --no-cache --purge \
        autoconf \
        automake \
        build-base \
        bash \
        curl \
        libmnl-dev \
        util-linux-dev \
        zlib-dev \
        libmnl \
        libuuid \
    && addgroup -g ${PGID} netdata \
    && adduser -D -H -u ${PUID} -G netdata netdata \
    && cd /tmp \
    && VERSION=$(curl -jksL https://api.github.com/repos/netdata/netdata/releases/latest | awk '/tag_name/{print $4;exit}' FS='[""]' | sed -e 's_^v__') \
	&& echo "Using version: ${VERSION}" \
    && curl -sSL https://github.com/netdata/netdata/releases/download/v${VERSION}/netdata-v${VERSION}.tar.gz | tar xz \
    && cd netdata-v${VERSION} \
    && ./netdata-installer.sh --dont-wait --dont-start-it --disable-telemetry \
    && apk del autoconf \
        automake \
        build-base \
        libmnl-dev \
        util-linux-dev \
        zlib-dev \
    && chown -R netdata:netdata /usr/share/netdata/web /var/lib/netdata /var/cache/netdata /var/log/netdata \
    && rm -rf /var/cache/apk/* /tmp/*

# Just adding a command to stop docker from complaining ..
CMD ["/bin/sh"]

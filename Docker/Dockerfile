FROM scratch

ARG TARGETPLATFORM

ADD binaries/$TARGETPLATFORM/gandi-live-dns /

WORKDIR /
ENTRYPOINT ["/gandi-live-dns"]
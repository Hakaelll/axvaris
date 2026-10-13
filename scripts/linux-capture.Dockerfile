FROM rust:1-bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6
RUN apt-get update
RUN apt-get install --no-install-recommends -y clang libbpf-dev iproute2 libcap2-bin
WORKDIR /workspace
ENV CARGO_TARGET_DIR=/workspace/target/linux-container

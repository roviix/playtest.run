# syntax=docker/dockerfile:1
#
# 服务端镜像：api 与 edge 两个二进制装在同一个镜像里，compose 用 command 选一个。
# 在本机构建（Apple Silicon 上 linux/arm64 是原生，几分钟），服务器只装载、不编译——
# 2 GB 内存的机器上编 Rust 会很慢，还可能被 OOM 杀掉。
#
#   docker buildx build --platform linux/arm64 -t playtest-server:<tag> --load .

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY common common
COPY api api
COPY edge edge
COPY cli cli
# 边缘把 SDK 编进二进制（edge/src/sdk.rs 的 include_bytes!），产物随源码进仓库。
COPY sdk/dist sdk/dist
# registry 与 target 都放在 BuildKit 缓存里：改一行代码只重编改动的 crate，不重下依赖。
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p playtest-api -p playtest-edge \
    && mkdir -p /out \
    && cp target/release/playtest-api target/release/playtest-edge /out/

FROM debian:bookworm-slim
# fonts-noto-cjk：邀请卡上的中文由边缘自己排版（edge/src/card.rs），
# 镜像里一个中文字体都没有的话，卡上会是一排方块。
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tini fonts-noto-cjk \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin playtest \
    && mkdir -p /data && chown playtest:playtest /data
COPY --from=build /out/playtest-api /out/playtest-edge /usr/local/bin/
USER playtest
WORKDIR /data
ENTRYPOINT ["tini", "--"]
CMD ["playtest-edge"]

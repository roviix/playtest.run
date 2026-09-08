# syntax=docker/dockerfile:1
#
# 官方 caddy 镜像不带 DNS 插件，而 *.playtest.run 的泛域名证书只能走 DNS-01，
# 所以用 xcaddy 编一个带 Cloudflare 模块的 caddy 再放回 alpine 底座。
#
#   docker buildx build --platform linux/arm64 -t playtest-caddy:<tag> --load -f deploy/caddy.Dockerfile deploy

FROM caddy:2-builder AS builder
RUN xcaddy build --with github.com/caddy-dns/cloudflare

FROM caddy:2-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy

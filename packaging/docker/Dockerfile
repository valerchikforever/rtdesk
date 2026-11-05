# Bridge servers (rendezvous + relay). Distroless, non-root, read-only friendly.
FROM rust:1.99-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p bridge-rendezvous -p bridge-relay

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/bridge-rendezvous /usr/local/bin/bridge-rendezvous
COPY --from=build /src/target/release/bridge-relay /usr/local/bin/bridge-relay
USER nonroot
VOLUME ["/data"]
EXPOSE 21116/udp 21117/udp

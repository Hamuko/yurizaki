# BUILD CONTAINER

FROM rust:1.86 AS build

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN USER=root cargo new --bin yurizaki

WORKDIR /yurizaki
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release

RUN rm src/*.rs
RUN rm ./target/release/deps/yurizaki*

ADD . ./

RUN cargo build --release


# RUNTIME CONTAINER

FROM debian:bookworm-slim

RUN groupadd -g 1000 yurizaki && \
    useradd -g yurizaki yurizaki

WORKDIR /home/yurizaki/bin/

COPY --from=build /yurizaki/target/release/yurizaki .
RUN chown yurizaki:yurizaki yurizaki

USER yurizaki

ENV RUST_LOG=info

CMD ["./yurizaki", "/config.yml"]

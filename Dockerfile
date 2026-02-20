# BUILD CONTAINER

FROM rust:1.93 AS build

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

FROM gcr.io/distroless/cc-debian13

COPY --from=build /yurizaki/target/release/yurizaki /

ENV RUST_LOG=info

CMD ["/yurizaki", "/config.yml"]

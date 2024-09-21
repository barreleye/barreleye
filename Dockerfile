FROM rust:1.81 as build
RUN USER=root cargo new --bin barreleye
WORKDIR /barreleye
COPY ./ ./
RUN cargo build --release

FROM debian:bookworm
COPY --from=build /barreleye/target/release/barreleye .

CMD ["./barreleye"]
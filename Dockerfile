FROM rust:1.35.0-stretch

WORKDIR /etc/sysinfo-web

ADD . .

RUN cargo build --release

EXPOSE 3000

CMD ./target/release/sysinfo-web

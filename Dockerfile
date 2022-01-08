FROM rust:1.57 as build

WORKDIR /app
COPY . .

RUN cargo build --release

FROM rust:1.57
RUN mkdir /app
COPY --from=build /app/target/release/crypto-quote-server /app

EXPOSE 80

CMD ["/app/crypto-quote-server", "-p", "80"]

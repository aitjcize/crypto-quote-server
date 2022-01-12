FROM rustlang/rust:nightly as build

WORKDIR /app
COPY . .

RUN cargo build --release

FROM rustlang/rust:nightly
WORKDIR /app

COPY --from=build /app/target/release/crypto-quote-server /app
COPY --from=build /app/Rocket.toml /app

EXPOSE 80

CMD ["/app/crypto-quote-server", "-p", "80"]

FROM rust:alpine as base
RUN apk add --no-cache musl-dev openssl-dev
ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN cargo install cargo-chef

FROM base as planner
WORKDIR app
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM base as cacher
WORKDIR app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM base as builder
WORKDIR app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release

FROM alpine
RUN apk add --no-cache libgcc tzdata
COPY --from=builder /app/target/release/nag /usr/local/bin/nag
ENTRYPOINT ["/usr/local/bin/nag"] 

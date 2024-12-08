##################
### BASE STAGE ###
##################
FROM rust:1.83.0 as base

# Install build dependencies
RUN rustup target add wasm32-unknown-unknown
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install --locked --version 0.0.3  strip_cargo_version
RUN cargo install --locked --version 0.18.7 trunk
RUN apt-get update &&\
    apt-get install -y musl-tools &&\
    apt-get clean &&\
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
RUN mkdir frontend backend lib

###########################
### STRIP-VERSION STAGE ###
###########################
FROM base AS strip-version

# Generate workspace manifest without ultrascroper
RUN tee Cargo.toml <<EOF
[workspace]
members = ["backend", "frontend", "lib"]
resolver = "2"
EOF

COPY Cargo.lock ./
COPY frontend/Cargo.toml ./frontend/
COPY backend/Cargo.toml ./backend/
COPY lib/Cargo.toml ./lib/
RUN strip_cargo_version

###################
### BUILD STAGE ###
###################
FROM base AS build

RUN cargo init --lib frontend
RUN cargo init --bin backend
RUN cargo init --lib lib

COPY --from=strip-version /app/frontend/Cargo.toml        /app/frontend/
COPY --from=strip-version /app/backend/Cargo.toml         /app/backend/
COPY --from=strip-version /app/lib/Cargo.toml             /app/lib/
COPY --from=strip-version /app/Cargo.lock /app/

WORKDIR /app/backend
RUN cargo build --release --target x86_64-unknown-linux-musl

WORKDIR /app/frontend
RUN cargo build --release --target wasm32-unknown-unknown

WORKDIR /app
COPY . .

WORKDIR /app/backend
RUN cargo build --release --target x86_64-unknown-linux-musl

WORKDIR /app/frontend
RUN trunk build --release

########################
### PRODUCTION STAGE ###
########################
FROM scratch

# Default logging level
ENV RUST_LOG="info"

ENV COVERS_DIR="/covers"
VOLUME /covers

WORKDIR /

# Copy application binary
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/singit_srv /usr/local/bin/singit_srv

# Copy static web files
COPY --from=build /app/frontend/dist /dist

# Copy database migrations
COPY backend/migrations /migrations

ENTRYPOINT ["singit_srv"]

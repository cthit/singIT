##################
### BASE STAGE ###
##################
FROM rust:1.72.1 as base

# Install build dependencies
RUN cargo install --locked trunk strip_cargo_version
RUN rustup target add wasm32-unknown-unknown
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
RUN mkdir frontend backend common

###########################
### STRIP-VERSION STAGE ###
###########################
FROM base AS strip-version

COPY Cargo.lock Cargo.toml ./
COPY frontend/Cargo.toml ./frontend/
COPY backend/Cargo.toml ./backend/
#COPY common/Cargo.toml ./common/
RUN strip_cargo_version

###################
### BUILD STAGE ###
###################
FROM base AS build

RUN cargo init --lib frontend
RUN cargo init --bin backend
RUN cargo init --lib common

COPY --from=strip-version /app/frontend/Cargo.toml /app/frontend/
COPY --from=strip-version /app/backend/Cargo.toml /app/backend/
#COPY --from=strip-version /app/common/Cargo.toml /app/common/
COPY --from=strip-version /app/Cargo.toml /app/Cargo.lock /app/

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

##################
### BASE STAGE ###
##################
FROM rust:1.60 as base

# Install build dependencies
RUN cargo install --locked trunk strip_cargo_version
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
RUN mkdir frontend backend common

###########################
### STRIP-VERSION STAGE ###
###########################
FROM base AS strip-version

COPY Cargo.lock Cargo.toml ./
RUN strip_cargo_version

###################
### BUILD STAGE ###
###################
FROM base AS build

RUN cargo init --lib .
COPY --from=strip-version /app/Cargo.toml /app/Cargo.lock /app/

RUN cargo build --release --target wasm32-unknown-unknown

COPY . .

RUN trunk build --release

########################
### PRODUCTION STAGE ###
########################
FROM nginx:alpine

COPY --from=base /app/dist/* /usr/share/nginx/html/

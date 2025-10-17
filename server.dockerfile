# GROUP: 42
# MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

# server.dockerfile
FROM rust:1.81-slim AS builder

RUN rustup install nightly && rustup default nightly

WORKDIR /app

EXPOSE 3000
CMD ["cargo", "run", "--manifest-path", "server/Cargo.toml"]

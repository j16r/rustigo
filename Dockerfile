FROM rustlang/rust:nightly AS build
WORKDIR /usr/src/rustigo

# Copy the source and build the application.
COPY . .
RUN cargo build --release

# Copy the statically-linked binary into a final minimal container
# FROM scratch
FROM debian:buster-slim

WORKDIR /opt
COPY --from=build /usr/src/rustigo/target/release/rustigo .
COPY templates ./templates
COPY Rocket.toml .

USER 1000
EXPOSE 8080
CMD ["./rustigo"]

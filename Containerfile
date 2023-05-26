FROM docker.io/rust:1.67 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --quiet --path .

FROM docker.io/ubuntu:rolling
RUN apt-get update -qq && apt-get install -qq -o=Dpkg::Use-Pty=0 -y ffmpeg yt-dlp > /dev/null 2>&1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/gpsp-bot /usr/local/bin/myapp
CMD ["myapp"]

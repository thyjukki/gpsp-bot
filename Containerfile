FROM docker.io/rust:1.67 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --quiet --path .

FROM docker.io/ubuntu:rolling
RUN apt-get -qq -o=Dpkg::Use-Pty=0 update && apt-get -qq -o=Dpkg::Use-Pty=0 install -y ffmpeg yt-dlp && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/gpsp-bot /usr/local/bin/myapp
CMD ["myapp"]

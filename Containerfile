FROM docker.io/rust:1.67 as builder
RUN apt-get update && apt-get install build-essential git zip python3 -y > /dev/null 2>&1
WORKDIR /usr/src/myapp

COPY . .

RUN cargo build --release
RUN cd /tmp && git clone https://github.com/yt-dlp/yt-dlp --depth=1
RUN cd /tmp/yt-dlp && make yt-dlp

FROM docker.io/debian:bookworm-slim

RUN apt-get update && apt-get install python3 -y > /dev/null 2>&1 && apt-get clean

COPY --from=builder /usr/src/myapp/target/release/gpsp-bot /usr/local/bin/myapp
COPY --from=builder /tmp/yt-dlp/yt-dlp /usr/local/bin/yt-dlp
COPY --from=mwader/static-ffmpeg:6.0 /ffmpeg /usr/local/bin/
COPY --from=mwader/static-ffmpeg:6.0 /ffprobe /usr/local/bin/

CMD ["myapp"]

FROM docker.io/golang:1.23.6

RUN apt-get update && \
  apt-get install ffmpeg python3 python3-pip -y && \
  go run github.com/playwright-community/playwright-go/cmd/playwright@v0.4802.0 install --with-deps chromium && \
  apt-get clean

WORKDIR /app
COPY . .

RUN go build .

ARG YOUTUBE_DLP_VERSION

RUN pip install --break-system-packages -U "yt-dlp[default]==${YOUTUBE_DLP_VERSION}"

CMD ["./gpsp-bot", "telegram"]
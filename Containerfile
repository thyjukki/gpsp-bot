FROM docker.io/golang:1.23.6
RUN apt-get update && \
  apt-get install ffmpeg yt-dlp -y && \
  go run github.com/playwright-community/playwright-go/cmd/playwright@v0.4802.0 install --with-deps chromium && \
  apt-get clean

WORKDIR /app
COPY . .

RUN go build .
CMD ["./gpsp-bot", "telegram"]
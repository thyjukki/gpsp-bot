FROM docker.io/golang:1.23.6
RUN apt-get update && \
  apt-get install ffmpeg python3 python3-pip -y && \
  go run github.com/playwright-community/playwright-go/cmd/playwright@v0.4802.0 install --with-deps chromium && \
  apt-get clean

WORKDIR /app
COPY . .

RUN go build .

RUN pip install --break-system-packages -U "yt-dlp[default]"

CMD ["./gpsp-bot", "telegram"]
# Dockerfile
FROM debian:stable 
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    rustup \
    pkg-config
RUN rustup toolchain install stable
RUN apt-get install -y \
    libgl-dev \
    libx11-xcb-dev \
    libxcursor-dev \
    libasound2-dev \
    python3 \
    libjack-dev \
    libxcb-icccm4-dev \
    libxcb-icccm4-dev \
    libxcb-dri2-0-dev

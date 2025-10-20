FROM mcr.microsoft.com/playwright:v1.56.1-focal

# Install system dependencies required for building the Rust GUI binary and running it
# behind a virtual framebuffer. These match the GTK/WebKit stack that Tauri and eframe
# rely on when running inside a container.
USER root
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    libatk-bridge2.0-dev \
    libatspi2.0-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    libgl1-mesa-dev \
    libegl1 \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libx11-dev \
    libx11-xcb-dev \
    libxi-dev \
    libxrandr-dev \
    librsvg2-dev \
    netcat-openbsd \
    pkg-config \
    xvfb \
  && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain and tauri-driver for talking to the GUI during tests.
USER pwuser
WORKDIR /home/pwuser
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --profile minimal --default-toolchain stable && \
    ~/.cargo/bin/rustup component add rustfmt clippy && \
    ~/.cargo/bin/cargo install tauri-driver --locked

ENV PATH="/home/pwuser/.cargo/bin:${PATH}"
WORKDIR /work

# Default to an interactive shell so the helper script can be invoked via docker run.
ENTRYPOINT ["/bin/bash"]

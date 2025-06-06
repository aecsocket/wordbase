FROM archlinux:base-devel

# setup dev user
RUN useradd -m -s /usr/bin/fish dev && \
    echo "dev ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/dev
USER dev
RUN mkdir -p "$HOME/.vscode-server"

# setup dev tools
RUN sudo pacman -Syu --noconfirm \
    bat exa fish fzf git gnupg helix just less man-db openssh ripgrep starship typos unzip yq zellij zoxide && \
    git clone https://aur.archlinux.org/yay-bin.git /tmp/yay && \
    cd /tmp/yay && \
    makepkg -si --noconfirm
COPY config.fish /etc/fish/config.fish
ENV EDITOR=/usr/bin/helix

# install Rust
RUN yay -Syu --noconfirm \
    rustup clang lld taplo && \
    rustup default stable && \
    rustup install nightly && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh && \
    cargo binstall -y cargo-shear

# install extra tools
RUN yay -Syu --noconfirm websocat sqlite && \
    cargo binstall -y sqlx-cli

# install UI development libraries
RUN yay -Syu --noconfirm \
    libadwaita webkitgtk-6.0 \
    meson blueprint-compiler python-sphinx-furo blueprint-compiler-docs-git \
    noto-fonts noto-fonts-cjk noto-fonts-emoji

# install audio playback libraries
RUN yay -Syu --noconfirm \
    alsa-utils pipewire pipewire-alsa \
    gstreamer gst-libav gst-plugins-base gst-plugins-good gst-plugins-bad

# install GNOME extension development tools
RUN yay -Syu --noconfirm npm gnome-shell zip

# Docker outside of Docker
RUN yay -Syu --noconfirm docker socat && \
    sudo usermod -aG docker dev

# Rust -> Kotlin binding tools
RUN yay -Syu --noconfirm ktlint

FROM archlinux:base-devel

# install dev container tools
RUN pacman -Syu --noconfirm \
    git openssh gnupg fish starship less ripgrep fzf bat exa zoxide yq just zellij helix && \
    chsh -s /usr/bin/fish root
COPY config.fish /etc/fish/config.fish
ENV EDITOR=/usr/bin/helix

# install Rust tools and libraries
RUN pacman -Syu --noconfirm rustup && \
    rustup default stable && \
    rustup install nightly && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh && \
    cargo binstall cargo-shear

# install extra tools
RUN pacman -Syu --noconfirm websocat

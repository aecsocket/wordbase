FROM archlinux:base-devel

# setup dev container
RUN pacman -Syu --noconfirm \
    git openssh gnupg fish starship man-db less ripgrep fzf bat exa zoxide yq just zellij helix && \
    useradd -m -s /usr/bin/fish dev && \
    echo "dev ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/dev
USER dev
RUN git clone https://aur.archlinux.org/yay-bin.git /tmp/yay && \
    chown -R dev:dev /tmp/yay && \
    cd /tmp/yay && \
    makepkg -si --noconfirm
COPY config.fish /etc/fish/config.fish
ENV EDITOR=/usr/bin/helix

# install Rust tools and libraries
RUN yay -Syu --noconfirm rustup && \
    rustup default stable && \
    rustup install nightly && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh && \
    cargo binstall cargo-shear

# install extra tools
RUN yay -Syu --noconfirm websocat mecab

USER root

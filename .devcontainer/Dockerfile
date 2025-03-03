FROM archlinux:base-devel

# setup dev user
RUN useradd -m -s /usr/bin/fish dev && \
    echo "dev ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/dev
USER dev
RUN mkdir -p "$HOME/.vscode-server"

# setup dev tools
RUN sudo pacman -Syu --noconfirm \
    git openssh gnupg fish starship man-db less ripgrep fzf unzip bat exa zoxide yq just zellij helix && \
    git clone https://aur.archlinux.org/yay-bin.git /tmp/yay && \
    cd /tmp/yay && \
    makepkg -si --noconfirm
COPY config.fish /etc/fish/config.fish
ENV EDITOR=/usr/bin/helix

# install Rust
RUN yay -Syu --noconfirm \
    rustup clang lld && \
    rustup default stable && \
    rustup install nightly && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh && \
    cargo binstall cargo-shear

# install extra tools
RUN yay -Syu --noconfirm websocatmecab && \
    sudo mkdir -p /usr/libexec && \
    sudo ln -s /usr/lib/mecab/ /usr/libexec/mecab && \
    yay -Syu --noconfirm mecab-ipadic

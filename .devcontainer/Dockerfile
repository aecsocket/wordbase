FROM mcr.microsoft.com/vscode/devcontainers/base:debian

RUN apt-get update -y && apt-get install -y direnv && \
    curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install linux \
    --extra-conf "sandbox = false" \
    --init none \
    --no-confirm && \
    chown -R vscode:vscode /nix && \
    echo 'export PATH="$PATH:/nix/var/nix/profiles/default/bin"' > /etc/profile.d/98-nix-path.sh
RUN echo 'eval $(cd /workspaces/* && direnv exec / direnv export bash)' > /etc/profile.d/99-nix-shell.sh

USER vscode

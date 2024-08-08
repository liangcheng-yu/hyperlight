FROM dependabot/dependabot-script
RUN rustup toolchain install 1.78.0 && rustup default 1.78.0

{
  description = "Dev shell for ArceOS training repository";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          riscvMuslCc = pkgs.pkgsCross."riscv64-musl".stdenv.cc;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustup
              riscvMuslCc

              gnumake
              bashInteractive
              coreutils
              findutils
              gnugrep
              gnused
              gawk
              git
              curl
              wget
              vim

              qemu
              dosfstools
              util-linux
              sudo

              llvmPackages.libclang
              llvmPackages.bintools
              clang-tools
              pkg-config
              openssl
              zlib

              gdb
              jq
              iproute2
              bridge-utils

              dosfstools
            ];

            shellHook = ''
              set -e

              export RUSTUP_HOME="$PWD/.rustup"
              export CARGO_HOME="$PWD/.cargo"
              mkdir -p "$RUSTUP_HOME" "$CARGO_HOME"

              # Keep aligned with arceos/rust-toolchain.toml.
              TOOLCHAIN="nightly-2024-09-04"
              if ! rustup toolchain list | grep -q "$TOOLCHAIN"; then
                rustup toolchain install "$TOOLCHAIN" \
                  --profile minimal \
                  --component rust-src \
                  --component llvm-tools \
                  --component rustfmt \
                  --component clippy \
                  --target x86_64-unknown-none \
                  --target riscv64gc-unknown-none-elf \
                  --target aarch64-unknown-none \
                  --target aarch64-unknown-none-softfloat
              fi
              rustup default "$TOOLCHAIN" >/dev/null

              mkdir -p .nix-tools/bin
              export PATH="$PWD/.nix-tools/bin:$PATH"

              link_if_exists() {
                local src="$1"
                local dst="$2"
                if command -v "$src" >/dev/null 2>&1; then
                  ln -sf "$(command -v "$src")" ".nix-tools/bin/$dst"
                fi
              }

              # Match command names expected by this repository.
              link_if_exists llvm-objcopy rust-objcopy
              link_if_exists llvm-objdump rust-objdump
              link_if_exists gdb gdb-multiarch

              for t in gcc ar ranlib strip; do
                link_if_exists "riscv64-unknown-linux-musl-$t" "riscv64-linux-musl-$t"
              done

              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"

              echo "[devShell] Ready: Rust toolchain, QEMU, musl cross tools, grading commands available."

              exec fish
            '';
          };
        });
    };
}

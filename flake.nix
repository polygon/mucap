{
  inputs = {
    #nixpkgs.url = "github:/NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit overlays system; };
      rust-bin = (pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      });
      dev-deps = with pkgs; [
        rust-bin
        rust-analyzer
        rustfmt
        lldb
        cargo-geiger
        cargo-flamegraph
      ];
      build-deps = with pkgs; [ pkg-config python3 ];
      runtime-deps = with pkgs; [ rtmidi alsa-lib.dev libGL.dev xorg.libX11.dev jack2.dev xorg.libXcursor.dev xorg.libxcb libxcb-wm ];
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = dev-deps ++ build-deps ++ runtime-deps;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.alsa-lib ];
      };

    };
}


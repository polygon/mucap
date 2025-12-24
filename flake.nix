{
  inputs = {
    nixpkgs.url = "github:/NixOS/nixpkgs/nixos-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, naersk }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit overlays system; };
      rust-bin = (pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      });
      version = toString (import ./VERSION);
      dev-deps = with pkgs; [
        rust-bin
        rust-analyzer
        rustfmt
        lldb
        cargo-geiger
        cargo-flamegraph
        fpm
      ];
      build-deps = with pkgs; [ pkg-config python3 ];
      runtime-deps = with pkgs; [
        rtmidi
        alsa-lib.dev
        libGL.dev
        xorg.libX11.dev
        jack2.dev
        xorg.libXcursor.dev
        xorg.libxcb
        libxcb-wm
      ];
      naersk-lib = naersk.lib.${system}.override {
        cargo = rust-bin;
        rustc = rust-bin;
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = dev-deps ++ build-deps ++ runtime-deps;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.alsa-lib ];
      };

      packages.${system} = {
        mucap = naersk-lib.buildPackage {
          pname = "mucap";
          root = ./.;
          version = version;
          buildInputs = runtime-deps;
          nativeBuildInputs = build-deps;
          overrideMain = attrs: {
            postBuild = ''
              cargo xtask bundle mucap --release
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/bundled/mucap $out/bin/mucap
              mkdir -p $out/lib/vst3
              mkdir -p $out/lib/clap
              cp target/bundled/mucap.clap $out/lib/clap
              cp -r target/bundled/mucap.vst3 $out/lib/vst3
            '';
            #fixupPhase = ''
            #  wrapProgram $out/bin/${program_name} \
            #    --prefix LD_LIBRARY_PATH : ${
            #      pkgs.lib.makeLibraryPath runtime-deps
            #    }
            #'';
            #patchPhase = ''
            #   sed -i s/\"dynamic\"// Cargo.toml
            #'';
          };
        };
      };

    };
}


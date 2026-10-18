{
  description = "Sensei";

  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs?ref=26.05";
    };
    # supabase-cli only: 26.05 ships 2.100, which rejects supabase/config.toml's
    # [local_smtp] section.
    nixpkgs-unstable = {
      url = "github:nixos/nixpkgs/nixos-unstable";
    };
    # Schema deploy tool for `task db:setup`, built from the sibling checkout.
    dbd = {
      url = "path:/home/pavankatakam/Projects/dbd";
      inputs.nixpkgs.follows = "nixpkgs-unstable";
    };
  };

  outputs =
    { self, nixpkgs, nixpkgs-unstable, dbd, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        config = {
          allowUnfree = true;
        };
      };
      pkgsUnstable = import nixpkgs-unstable { inherit system; };
      # Native libraries the Tauri desktop shell links against on Linux.
      tauriLibs = with pkgs; [
        webkitgtk_4_1
        gtk3
        libsoup_3
        glib
        glib-networking
        cairo
        pango
        gdk-pixbuf
        atk
        librsvg
        libayatana-appindicator
        openssl
      ];
      # WebKitGTK's media stack ("GStreamer element appsink not found").
      gstPlugins = with pkgs.gst_all_1; [
        gstreamer
        gst-plugins-base
        gst-plugins-good
      ];
    in
    {
      formatter.${system} = pkgs.nixfmt;

      devShells.${system} = {

        default = pkgs.mkShell {
          name = "sensei";

          nativeBuildInputs = with pkgs; [
            pkg-config
            # libclang for bindgen (pg_query's build script)
            rustPlatform.bindgenHook
          ];

          buildInputs =
            with pkgs;
            [
              nixd
              nixfmt

              gnumake
              go-task
              process-compose
              git

              # Rust workspace (crates/ + app/src-tauri)
              rustc
              cargo
              clippy
              rustfmt
              rust-analyzer

              # SvelteKit frontends (app/, dojo/, website/)
              bun
              nodejs

              # psql / pg_dump for the daemon's bootstrap and `make db-backup`
              postgresql_18

              # Local Dōjō stack (supabase/config.toml) — `task dojo:up`
              pkgsUnstable.supabase-cli

              # Schema deploy (`task db:setup`)
              dbd.packages.${system}.dbd
            ]
            ++ tauriLibs
            ++ gstPlugins;

          shellHook = ''
            export PROJECT_ROOT=$PWD
            export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
            # Linuxbrew (sensei's release install, postgresql@17). Appended so the
            # shell's Nix tools keep precedence over same-named brew binaries.
            if [ -d /home/linuxbrew/.linuxbrew/bin ]; then
              export PATH="$PATH:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin"
            fi

            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath tauriLibs}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
            export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}
            export GIO_MODULE_DIR=${pkgs.glib-networking}/lib/gio/modules
            export GST_PLUGIN_SYSTEM_PATH_1_0=${pkgs.lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" gstPlugins}
            # WebKitGTK's DMA-BUF renderer shows a blank window on many
            # Linux GPU/driver combos outside a full desktop session.
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
            # Off NixOS there is no /run/opengl-driver, so nix's libglvnd finds no
            # EGL vendor and WebKit aborts ("Could not create default EGL display"),
            # leaving the transparent window empty. Point it at nixpkgs' mesa.
            export __EGL_VENDOR_LIBRARY_FILENAMES=${pkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json
            export LIBGL_DRIVERS_PATH=${pkgs.mesa}/lib/dri
            export GBM_BACKENDS_PATH=${pkgs.mesa}/lib/gbm
          '';
        };
      };
    };
}

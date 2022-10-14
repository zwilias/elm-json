{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/release-22.05"; };
  outputs = { self, nixpkgs }:
    let
      system = "aarch64-darwin";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShell.${system} = pkgs.mkShell {
        buildInputs = [
          pkgs.clippy
          pkgs.cargo
          # cargo build requires these
          pkgs.libiconv
          pkgs.curl
          # to release npm packages and whatnot
          pkgs.nodejs

        ];
      };
    };
}

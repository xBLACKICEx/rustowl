# Packages for NUR
{ pkgs ? import <nixpkgs> { } }:
let
  flake = builtins.getFlake (toString ./.);
  inherit (pkgs) system;
in
flake.packages."${system}"
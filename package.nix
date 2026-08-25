# TODO: move this to nixpkgs
# This file aims to be a replacement for the nixpkgs derivation.

{
  buildFeatures ? [ ],
  buildNoDefaultFeatures ? false,
  buildPackages,
  fetchFromGitHub,
  installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellFiles,
  lib,
  openssl,
  pkg-config,
  rustPlatform,
  stdenv,
}:

let
  nativeTls = builtins.elem "native-tls" buildFeatures;

in
rustPlatform.buildRustPackage (finalAttrs: {
  __structuredAttrs = true;

  inherit buildNoDefaultFeatures buildFeatures;

  pname = "himalaya-tui";
  version = "0.1.0";
  cargoHash = "";

  src = fetchFromGitHub {
    owner = "pimalaya";
    repo = finalAttrs.pname;
    tag = "v${finalAttrs.version}";
    hash = "";
  };

  # openssl should not be provided by vendors, not even on windows
  env.OPENSSL_NO_VENDOR = 1;

  nativeBuildInputs = [
    pkg-config
    installShellFiles
  ];

  buildInputs = lib.optional nativeTls openssl;

  postInstall =
    let
      exe =
        if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
          "$out/bin/${finalAttrs.pname}"
        else
          lib.getExe buildPackages.${finalAttrs.pname};
    in
    ''
      mkdir -p $out/share/{completions,man}
      ${exe} manuals -d "$out"/share/man
      ${exe} completions -d "$out"/share/completions bash elvish fish powershell zsh
    ''
    + lib.optionalString installManPages ''
      installManPage "$out"/share/man/*
    ''
    + lib.optionalString installShellCompletions ''
      installShellCompletion --cmd ${finalAttrs.pname} \
        --bash "$out"/share/completions/${finalAttrs.pname}.bash \
        --fish "$out"/share/completions/${finalAttrs.pname}.fish \
        --zsh "$out"/share/completions/_${finalAttrs.pname}
    '';

  # the crate ships no library target, so the unit tests all live in the
  # binary
  cargoTestFlags = [ "--bins" ];

  meta = {
    description = "TUI to manage emails";
    mainProgram = finalAttrs.pname;
    homepage = "https://github.com/pimalaya/${finalAttrs.pname}";
    changelog = "https://github.com/pimalaya/${finalAttrs.pname}/releases/${finalAttrs.src.tag}";
    license = with lib.licenses; [
      asl20
      mit
    ];
    maintainers = with lib.maintainers; [ soywod ];
  };
})

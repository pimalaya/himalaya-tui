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
  version = "0.1.0";
  emul = stdenv.hostPlatform.emulator buildPackages;
  exe = stdenv.hostPlatform.extensions.executable;

in
rustPlatform.buildRustPackage {
  inherit version buildNoDefaultFeatures buildFeatures;

  pname = "himalaya-tui";
  cargoHash = "";

  src = fetchFromGitHub {
    hash = "";
    owner = "pimalaya";
    repo = "himalaya-tui";
    rev = "v${version}";
  };

  env.OPENSSL_NO_VENDOR = true;

  nativeBuildInputs = [
    pkg-config
    installShellFiles
  ];

  buildInputs = lib.optional (builtins.elem "native-tls" buildFeatures) openssl;

  # most of the tests are lib side
  doCheck = false;

  postInstall =
    lib.optionalString (lib.hasInfix "wine" emul) ''
      export WINEPREFIX="''${WINEPREFIX:-$(mktemp -d)}"
      mkdir -p $WINEPREFIX
    ''
    + ''
      mkdir -p $out/share/{applications,completions,man}
      ${emul} "$out"/bin/himalaya-tui${exe} manuals "$out"/share/man
      ${emul} "$out"/bin/himalaya-tui${exe} completions -d "$out"/share/completions bash elvish fish powershell zsh
    ''
    + lib.optionalString installManPages ''
      installManPage "$out"/share/man/*
    ''
    + lib.optionalString installShellCompletions ''
      installShellCompletion --cmd himalaya-tui \
        --bash "$out"/share/completions/himalaya-tui.bash \
        --fish "$out"/share/completions/himalaya-tui.fish \
        --zsh "$out"/share/completions/_himalaya-tui
    '';

  meta = {
    description = "TUI to manage emails";
    mainProgram = "himalaya-tui";
    homepage = "https://github.com/pimalaya/himalaya-tui";
    changelog = "https://github.com/pimalaya/himalaya-tui/blob/master/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    maintainers = with lib.maintainers; [ soywod ];
  };
}

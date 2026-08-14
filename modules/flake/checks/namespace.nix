# Holds the module a consumer imports to reading nothing outside its own
# namespace.
#
# This is the property an adapter rests on. A consumer with an existing user
# registry bridges it by writing a projection into `flake.safix.users`, and that
# projection is sufficient only while safix reads nothing else: one option path
# read from somewhere else — a consumer's own users, a fleet-wide default, a
# hostname list — turns the adapter into an integration, and the consumer then
# has to reproduce a shape safix never documented.
#
# It is a grep rather than an evaluation because that is the shape of the claim.
# An evaluation shows what a particular fleet made the modules read; the claim is
# about every fleet, including the ones nobody has written, and an option path is
# a syntactic thing in this codebase.
#
# ── the drill ──
# The same script runs over a fixture tree carrying one offending read, and has
# to fail on it and name the file. Sharing the script is what makes the drill
# evidence about the check: a drill with its own copy of the grep would prove
# that its copy fires.
{
  lib,
  ...
}:
{
  perSystem =
    { pkgs, ... }:
    let
      # `config.flake.safix` is the whole of what the module may reach. The
      # module system's own `_module` arguments are not option reads and do not
      # appear; if one ever does, it belongs here with a reason beside it rather
      # than in a widened pattern.
      #
      # Comment-only lines are dropped before the match. Prose naming an option
      # path is not a read, and a check that could be satisfied by rewording a
      # sentence would be measuring the prose.
      permitted = "config\\.flake\\.safix";

      scan = pkgs.writeShellScript "safix-namespace-scan" ''
        set -eu
        tree="$1"
        offences=""
        for file in "$tree"/*.nix; do
          hits="$(grep -nE 'config\.[a-zA-Z]' "$file" \
            | grep -vE '^[0-9]+:[[:space:]]*#' \
            | grep -vE ${lib.escapeShellArg permitted} || true)"
          if [ -n "$hits" ]; then
            offences="$offences$(printf '%s\n' "$hits" | sed "s|^|$(basename "$file"):|")"$'\n'
          fi
        done
        if [ -n "$offences" ]; then
          {
            echo "safix namespace: a module reads an option path outside flake.safix."
            echo
            printf '%s' "$offences"
            echo
            echo "A consumer's adapter is a projection into flake.safix.users and"
            echo "flake.safix.catalogue. Reading anything else makes that projection"
            echo "insufficient, and the shape it would then have to supply is one"
            echo "safix never documented."
          } >&2
          exit 1
        fi
      '';

      offendingTree = pkgs.runCommand "safix-namespace-fixture" { } ''
        mkdir -p "$out"
        cat > "$out/adapter.nix" <<'FIXTURE'
        { config, ... }:
        {
          flake.safix.users = config.flake.users;
        }
        FIXTURE
      '';
    in
    {
      checks.safix-namespace =
        pkgs.runCommand "safix-namespace"
          {
            meta.description = "structural check: safix-namespace";
          }
          ''
            ${scan} ${../safix}
            touch "$out"
          '';

      checks.safix-drill-namespace =
        pkgs.runCommand "safix-drill-namespace"
          {
            meta.description = "structural check: safix-drill-namespace";
          }
          ''
            if ${scan} ${offendingTree} 2>report; then
              echo "the namespace scan exited 0 over a module reading config.flake.users" >&2
              exit 1
            fi
            if ! grep -qF "adapter.nix" report; then
              echo "the namespace failure does not name the file it found the read in" >&2
              cat report >&2
              exit 1
            fi
            touch "$out"
          '';
    };
}

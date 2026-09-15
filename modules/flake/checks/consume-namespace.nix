# Holds safix's consumption modules to defining and reading no option under
# another provisioner's `sops` namespace.
#
# safix installs its own secrets now, through its own program, and every option
# it declares or reads is `safix.*`. The claim that matters to a consumer is
# the negative one: importing a safix module leaves whatever they set under
# `sops.*` alone, because safix neither writes there nor reads from there. An
# option tree two modules define is an option tree either of them can surprise
# the other with, and a consumer who still runs sops-nix beside safix has to be
# able to read that from the code rather than from a promise.
#
# It is a grep rather than an evaluation, for `namespace.nix`'s reason: an
# evaluation shows what one fleet's configuration made the modules read, and
# the claim is about every fleet including the unwritten ones, where an option
# path is a syntactic thing in this tree.
#
# ── what is excluded, and by which spelling ──
# Two exact spellings are removed from each line before the match, rather than
# the pattern being widened to tolerate them. A widened pattern is a pattern
# that stops measuring: `sops` matched loosely would have to permit every
# spelling that merely begins that way, and the whole point is that
# `sopsCfg.age.keyFile` must not be among them.
#   - `${pkgs.sops}/bin/sops` — the sops binary, which safix invokes to decrypt
#     a document and which is a package reference, not an option.
#   - `sopsFile` — the field naming the document an entry is read out of. It is
#     safix's own field, in safix's own entry type, and it is spelled this way
#     because that is the name the format has.
# `sed` is what removes them, so every line number below is the file's own.
#
# ── what is matched ──
# The namespace is reached under more than one spelling, and the check has to
# name all of them or the drill that reintroduces one of them stays green. A
# `let` binding named `sopsCfg` is how these files used to hold
# `config.sops`, so the binding's own name is an option read as much as the
# path is — and it is the spelling the reintroduction drill uses, because it is
# the spelling the deleted code had.
{
  lib,
  ...
}:
{
  perSystem =
    { pkgs, ... }:
    let
      # The two excluded spellings, as `sed` expressions that delete them from
      # a line before it is matched. Every character the expression syntax
      # would otherwise read is bracket-quoted, which is literal in the basic
      # syntax `sed` uses and needs no backslash — a backslash before `{` there
      # opens an interval instead.
      #
      #   [$][{]pkgs[.]sops[}]/bin/sops  the sops binary safix invokes
      #   sopsFile                       safix's own field naming a document
      strip = "-e 's|[$][{]pkgs[.]sops[}]/bin/sops||g' -e 's|sopsFile||g'";

      # The four spellings of the namespace: the path read out of `config`, the
      # path declared under `options`, the `let` binding that used to hold the
      # first, and a definition or read of any attribute below `sops` itself.
      offending = "(^|[^[:alnum:]_])(config\\.sops|options\\.sops|sopsCfg|sops\\.[a-zA-Z]|sops[[:space:]]*=)";

      scan = pkgs.writeShellScript "safix-consume-namespace-scan" ''
        set -eu
        tree="$1"
        offences=""
        for file in "$tree"/*.nix; do
          hits="$(sed ${strip} "$file" \
            | grep -nE ${lib.escapeShellArg offending} \
            | grep -vE '^[0-9]+:[[:space:]]*#' || true)"
          if [ -n "$hits" ]; then
            offences="$offences$(printf '%s\n' "$hits" | sed "s|^|$(basename "$file"):|")"$'\n'
          fi
        done
        if [ -n "$offences" ]; then
          {
            echo "safix consumption: a module reads or defines an option under the sops namespace."
            echo
            printf '%s' "$offences"
            echo
            echo "safix installs its own secrets and declares every option it owns"
            echo "under safix.*. A consumer running sops-nix beside safix keeps every"
            echo "value they set there, which holds only while safix defines nothing"
            echo "in that tree and reads nothing out of it."
          } >&2
          exit 1
        fi
      '';

      # One offending read, spelled the way the deleted code spelled it, so the
      # empty answer above is evidence rather than the answer a pattern that
      # matches nothing would also give.
      offendingTree = pkgs.runCommand "safix-consume-namespace-fixture" { } ''
        mkdir -p "$out"
        cat > "$out/installer.nix" <<'FIXTURE'
        { config, ... }:
        let
          sopsCfg = config.sops;
        in
        {
          config.safix.installer.keepGenerations = sopsCfg.keepGenerations;
        }
        FIXTURE
      '';
    in
    {
      checks.safix-consume-namespace =
        pkgs.runCommand "safix-consume-namespace"
          {
            meta.description = "structural check: safix-consume-namespace";
          }
          ''
            ${scan} ${../../consume}
            touch "$out"
          '';

      checks.safix-drill-consume-namespace =
        pkgs.runCommand "safix-drill-consume-namespace"
          {
            meta.description = "structural check: safix-drill-consume-namespace";
          }
          ''
            if ${scan} ${offendingTree} 2>report; then
              echo "the consumption scan exited 0 over a module reading config.sops" >&2
              exit 1
            fi
            if ! grep -qF "installer.nix" report; then
              echo "the failure does not name the file it found the read in" >&2
              cat report >&2
              exit 1
            fi
            touch "$out"
          '';
    };
}

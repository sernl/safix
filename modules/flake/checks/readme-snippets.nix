# Holds every nix block of ../../../README.md against the file it names in
# ../../../examples/quickstart/.
#
# ── what a block has to carry ──
# A fenced block whose info string starts with `nix` must name its source as
# `title="examples/quickstart/<path>"`, optionally with `#<region>` appended.
# An untitled nix block fails, naming the README line it opens on: a snippet
# nothing resolves is a snippet nothing evaluated, which is the state this
# check exists to make impossible.
#
# ── how a region is resolved ──
# A region is the lines strictly between `# --8<-- [start:<region>]` and
# `# --8<-- [end:<region>]`, the mkdocs-snippets convention, so a site
# generator can include the identical range later. Marker lines are dropped
# from what is compared — from a whole file as well as from a region — and
# nothing else is. The comparison is a byte diff afterwards, which is the
# point: the example files are formatted by this repository's formatter and
# the README blocks are copied out of them, so whitespace drift is drift.
#
# ── why this is a derivation over two paths ──
# Its inputs are the README and the example tree and nothing else, so it
# re-runs when either moves and not when the rest of the repository does. It
# needs no nix evaluation of the example: `./examples-quickstart.nix` is the
# evaluation, and this one is about the text.
#
# ── severity: three drills, all observed ──
# Adding one space to a quoted line in the README fails the block, naming
# `README.md:<line>` and the source file. Observed.
# Dropping the `title=` from a nix block fails naming that block's line rather
# than passing it over. Observed.
# Naming a region no example file marks fails naming the region, which is what
# keeps a renamed region from silently comparing against nothing. Observed.
{
  perSystem =
    { pkgs, ... }:
    {
      checks.safix-readme-snippets =
        pkgs.runCommand "safix-readme-snippets"
          {
            readme = ../../../README.md;
            quickstart = ../../../examples/quickstart;
            nativeBuildInputs = [
              pkgs.gawk
              pkgs.diffutils
            ];
            meta.description = "every README nix block matches the example file it names";
          }
          ''
            set -euo pipefail
            mkdir -p blocks
            : > index

            # Split the README into one file per nix block. A fence in any
            # other language is skipped whole, so its closing fence is never
            # read as an opening one.
            awk '
              /^```/ {
                info = substr($0, 4)
                if (info ~ /^nix/) {
                  n++
                  out = sprintf("blocks/%03d.body", n)
                  printf "" > out
                  printf "%03d\t%d\t%s\n", n, NR, info >> "index"
                  while ((getline) > 0) {
                    if ($0 ~ /^```[ \t]*$/) break
                    print $0 >> out
                  }
                  close(out)
                } else {
                  while ((getline) > 0) {
                    if ($0 ~ /^```[ \t]*$/) break
                  }
                }
                next
              }
            ' "$readme"

            if [ ! -s index ]; then
              echo "readme-snippets: README.md has no nix block at all; the extractor read nothing."
              exit 1
            fi

            extract() {
              local file="$1" region="$2"
              if [ -z "$region" ]; then
                awk '!/^[[:space:]]*# --8<-- \[(start|end):/' "$file"
              else
                awk -v r="$region" '
                  $0 ~ "^[[:space:]]*# --8<-- \\[start:" r "\\][[:space:]]*$" { inside = 1; found = 1; next }
                  $0 ~ "^[[:space:]]*# --8<-- \\[end:" r "\\][[:space:]]*$" { inside = 0; next }
                  inside && /^[[:space:]]*# --8<-- \[(start|end):/ { next }
                  inside { print }
                  END { if (!found) exit 3 }
                ' "$file"
              fi
            }

            status=0
            while IFS="$(printf '\t')" read -r num lineno info; do
              body="blocks/$num.body"
              title=$(printf '%s\n' "$info" | sed -n 's/.*title="\([^"]*\)".*/\1/p')

              if [ -z "$title" ]; then
                echo "README.md:$lineno: nix block carries no title=\"examples/quickstart/<path>\"."
                status=1
                continue
              fi

              case "$title" in
                examples/quickstart/*) ;;
                *)
                  echo "README.md:$lineno: title \"$title\" names no file under examples/quickstart/."
                  status=1
                  continue
                  ;;
              esac

              rest=''${title#examples/quickstart/}
              region=""
              case "$rest" in
                *"#"*)
                  region=''${rest#*#}
                  rest=''${rest%%#*}
                  ;;
              esac

              source="$quickstart/$rest"
              if [ ! -f "$source" ]; then
                echo "README.md:$lineno: examples/quickstart/$rest does not exist."
                status=1
                continue
              fi

              if ! extract "$source" "$region" > expected; then
                echo "README.md:$lineno: examples/quickstart/$rest marks no region '$region'."
                status=1
                continue
              fi

              if ! diff -u --label "$title" --label "README.md:$lineno" expected "$body"; then
                if [ -n "$region" ]; then
                  echo "README.md:$lineno: block differs from region '$region' of examples/quickstart/$rest."
                else
                  echo "README.md:$lineno: block differs from examples/quickstart/$rest."
                fi
                status=1
              fi
            done < index

            if [ "$status" -ne 0 ]; then
              echo ""
              echo "readme-snippets: a README nix block and the example it names disagree."
              echo "Copy the example file's region into the block, or fix the example and rerun."
              exit 1
            fi

            touch $out
          '';
    };
}

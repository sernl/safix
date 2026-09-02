# `onboardingHook` and `enrollHook`, declared here and merged into ./entry.nix's
# `safix` attrset directly rather than returned by `mkVault` — design decision
# D4: they are siblings of `flake.safix.lib`, not fields inside it, and
# `mkVault` returns only the `.lib` half.
{
  onboardingHook = ''
    name="$1"
    recipient="$2"
    printf 'onboarded %s (%s) — attach an account or a host import by hand in this example\n' "$name" "$recipient"
  '';

  # Unset is a supported configuration: `safix enroll` succeeds without a hook,
  # having done less.
  enrollHook = null;
}

# The two hooks, declared here so both consumers carry them and the check
# compares one side's declaration against the other's rather than one file
# against itself. The values are `examples/plain-nix/hooks.nix`'s.
{
  flake.safix.onboardingHook = ''
    name="$1"
    recipient="$2"
    printf 'onboarded %s (%s) — attach an account or a host import by hand in this example\n' "$name" "$recipient"
  '';

  # Unset is a supported configuration: `safix enroll` succeeds without a hook,
  # having done less.
  flake.safix.enrollHook = null;
}

# Every declaration the quickstart makes: one person, one host, two entries and
# one rotation policy. `./flake.nix` hands this file to `mkVault`, and nothing
# else in this tree declares custody.
#
# The regions marked below are the blocks `README.md` quotes. A block there and
# the region here are compared byte for byte, so editing one without the other
# fails `safix-readme-snippets`.
{
  # --8<-- [start:declare]
  flake.safix.users.alice = {
    recipient = "age1exampleaaa00000000000000000000000000000000000000000000000";
    recipientNote = "alice — example identity, decrypts nothing";

    # A value a person types. Declaring it says alice holds it and nobody else
    # does; the grant below hands the same name to the machine, so the file is
    # encrypted to alice and to web and to no one further.
    private."grafana-admin-password".restartUnits = [ "grafana.service" ];

    sharedWith.web."grafana-admin-password" = { };
  };

  flake.safix.machines.web = {
    recipient = "age1exampleweb00000000000000000000000000000000000000000000000";
    recipientNote = "web — the age form of a host identity that does not exist";
    owner = "alice";
  };
  # --8<-- [end:declare]

  # --8<-- [start:generator]
  flake.safix.users.alice.private."grafana-secret-key" = {
    generator = {
      script = ''openssl rand -hex 32 > "$out/grafana-secret-key"'';
      runtimeInputs = [ "openssl" ];
      validation = "grep -Eq '^[0-9a-f]{64}$' ";
    };

    rotation = "quarterly";
    restartUnits = [ "grafana.service" ];
  };

  flake.safix.users.alice.sharedWith.web."grafana-secret-key" = { };
  # --8<-- [end:generator]

  # --8<-- [start:policy]
  flake.safix.rotation.quarterly.every = "90d";
  # --8<-- [end:policy]
}

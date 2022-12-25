use assert_cmd::Command;

#[tokio::test]
pub async fn cli_should_show_help_text() {
    let mut cmd = Command::cargo_bin("vrrb").unwrap();
    let help_text = r#"Usage: vrrb [OPTIONS] [COMMAND]

Commands:
  node         Node management subcommands
  wallet       Wallet management subcommands
  placeholder  Placeholder sub-command to demonstrate how to configure them
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --config <FILE>  Sets a custom config file
  -d, --debug...       Turn debugging information on
  -h, --help           Print help information
  -V, --version        Print version information
"#;

    cmd.arg("--help").assert().stdout(help_text).success();
}

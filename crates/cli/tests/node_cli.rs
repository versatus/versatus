use assert_cmd::Command;

#[tokio::test]
// #[ignore = "temporarily broken because of GH actions issue"]
pub async fn cli_should_show_help_text() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    let help_text = r#"cli 0.0.0

USAGE:
    cli [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --config <FILE>    Sets a custom config file
    -d, --debug            Turn debugging information on
    -h, --help             Print help information
    -V, --version          Print version information

SUBCOMMANDS:
    help           Print this message or the help of the given subcommand(s)
    node           Node management subcommands
    placeholder    Placeholder sub-command to demonstrate how to configure them
"#;

    cmd.arg("--help").assert().stdout(help_text).success();
}

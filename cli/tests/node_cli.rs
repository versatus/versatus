use assert_cmd::Command;

#[tokio::test]
pub async fn cli_should_show_help_text() {
    let mut cmd = Command::cargo_bin("cli");

    cmd.assert();
}

//! Launches the system's RDP client — a "launcher" integration, not an
//! embedded renderer (see the `HostKind::Rdp` doc comment in `model.rs` and
//! the roadmap notes in `CLAUDE.md`: rendering RDP inside the app is a
//! separate, much larger project). Windows uses `mstsc.exe` against a
//! temporary `.rdp` file, with credentials staged via `cmdkey` so the
//! session opens without retyping the password. Linux uses `xfreerdp` /
//! `xfreerdp3` if present on `PATH`. macOS isn't supported yet — there's no
//! scriptable native client to shell out to.

/// Launches an RDP session to `address:port` as `username`, using `password`
/// if given (the client prompts for credentials itself otherwise). Returns
/// once the client process has been *started* — the session itself runs
/// independently of this app, in its own window.
pub async fn launch(address: &str, port: u16, username: &str, password: Option<&str>) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    return windows::launch(address, port, username, password).await;

    #[cfg(target_os = "linux")]
    return linux::launch(address, port, username, password).await;

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (address, port, username, password);
        anyhow::bail!("Le lancement RDP n'est pas encore pris en charge sur cette plateforme");
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::process::Stdio;
    use tokio::process::Command;

    /// Windows Credential Manager target name for `address` — shared between
    /// staging the credential before launch and removing it after, and
    /// pulled out so both call sites (and the test below) always agree on
    /// the format.
    fn cred_target(address: &str) -> String {
        format!("TERMSRV/{address}")
    }

    /// Contents of the temporary `.rdp` file handed to `mstsc.exe`.
    /// `authentication level:i:0` connects through an unverified/self-signed
    /// certificate without an interstitial warning — reasonable for a quick
    /// launcher aimed at internal hosts, but a deliberate trade-off worth
    /// keeping in mind if this ever grows a certificate-trust story of its
    /// own.
    fn rdp_file_contents(address: &str, port: u16, username: &str) -> String {
        format!(
            "full address:s:{address}:{port}\nusername:s:{username}\nprompt for credentials:i:0\nauthentication level:i:0\n"
        )
    }

    pub async fn launch(address: &str, port: u16, username: &str, password: Option<&str>) -> anyhow::Result<()> {
        let target = cred_target(address);

        if let Some(pw) = password {
            let status = Command::new("cmdkey")
                .arg(format!("/generic:{target}"))
                .arg(format!("/user:{username}"))
                .arg(format!("/pass:{pw}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("cmdkey n'a pas pu enregistrer les identifiants");
            }
        }

        let path = std::env::temp_dir().join(format!("gui-termius-rdp-{}.rdp", uuid::Uuid::new_v4()));
        tokio::fs::write(&path, rdp_file_contents(address, port, username)).await?;

        let mut child = match Command::new("mstsc.exe").arg(&path).spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(e.into());
            }
        };

        let has_password = password.is_some();
        tokio::spawn(async move {
            let _ = child.wait().await;
            if has_password {
                let _ = Command::new("cmdkey")
                    .arg(format!("/delete:{target}"))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
            }
            let _ = tokio::fs::remove_file(&path).await;
        });

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cred_target_matches_windows_credential_manager_convention() {
            assert_eq!(cred_target("10.0.4.12"), "TERMSRV/10.0.4.12");
        }

        #[test]
        fn rdp_file_contains_address_port_and_username() {
            let contents = rdp_file_contents("192.168.1.42", 3389, "alice");
            assert!(contents.contains("full address:s:192.168.1.42:3389"));
            assert!(contents.contains("username:s:alice"));
            assert!(contents.contains("prompt for credentials:i:0"));
        }

        /// Real smoke test, not run by default (`cargo test -- --ignored`):
        /// launches an actual `mstsc.exe` against a TEST-NET-3 address
        /// (RFC 5737 — guaranteed non-routable, so this can never reach a
        /// real host) and checks that `cmdkey` staged and then cleaned up a
        /// credential, and that the temp `.rdp` file was removed afterwards.
        /// Leaves an `mstsc.exe` window open for the user to close (it can't
        /// connect to a TEST-NET-3 address, so it just sits on an error).
        #[tokio::test]
        #[ignore]
        async fn launch_spawns_mstsc_and_cleans_up_afterwards() {
            let target = cred_target("203.0.113.42");
            super::launch("203.0.113.42", 3389, "test-user", Some("test-pass"))
                .await
                .expect("launch should spawn mstsc.exe");

            let staged = Command::new("cmdkey")
                .arg("/list")
                .output()
                .await
                .expect("cmdkey /list should run");
            let staged_out = String::from_utf8_lossy(&staged.stdout);
            assert!(staged_out.contains(&target), "expected {target} to be staged in Credential Manager while mstsc runs");

            println!("mstsc.exe launched against a TEST-NET-3 address — close the window it opened, then re-run to confirm cleanup, or just inspect Credential Manager for {target} disappearing once mstsc exits.");
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::process::Stdio;
    use tokio::process::Command;

    pub async fn launch(address: &str, port: u16, username: &str, password: Option<&str>) -> anyhow::Result<()> {
        let client = find_client().await?;
        let mut cmd = Command::new(client);
        cmd.arg(format!("/v:{address}:{port}"));
        cmd.arg(format!("/u:{username}"));
        if let Some(pw) = password {
            cmd.arg(format!("/p:{pw}"));
        }
        cmd.arg("/cert:ignore");
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.spawn()?;
        Ok(())
    }

    async fn find_client() -> anyhow::Result<&'static str> {
        for candidate in ["xfreerdp3", "xfreerdp"] {
            let found = Command::new("which")
                .arg(candidate)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false);
            if found {
                return Ok(candidate);
            }
        }
        anyhow::bail!("Aucun client RDP trouvé sur ce système (xfreerdp non installé)")
    }
}

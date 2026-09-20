pub mod azure;
pub mod copilot;
pub mod openai;
pub mod openrouter;

use anyhow::{Context, Result};
use colored::Colorize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{self, Read, Write};
use std::process::Command;

pub fn prompt_to_copy_device_code(code: &str) {
    println!(
        "\nPress {} to copy the code, or {} to continue...",
        "'c'".cyan().bold(),
        "Enter".cyan().bold()
    );

    let mut copied = false;
    if enable_raw_mode().is_ok() {
        let mut buf = [0u8; 1];
        if io::stdin().lock().read_exact(&mut buf).is_ok() {
            copied = (buf[0] as char).eq_ignore_ascii_case(&'c');
        }
        let _ = disable_raw_mode();
    }

    if copied {
        match copy_to_clipboard(code) {
            Ok(()) => println!("\n{} Code copied to clipboard!", "✓".green()),
            Err(error) => println!("\n{} Failed to copy to clipboard: {}", "⚠️".yellow(), error),
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("pbcopy");
        cmd.stdin(std::process::Stdio::piped());
        let mut child = cmd.spawn().context("Failed to spawn pbcopy")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait().context("Failed to copy to clipboard")?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("powershell")
            .arg("-command")
            .arg("Set-Clipboard")
            .arg("-Value")
            .arg(text)
            .status()
            .context("Failed to copy to clipboard")?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        if Command::new("wl-copy")
            .arg(text)
            .status()
            .is_ok_and(|status| status.success())
        {
            return Ok(());
        }

        let mut cmd = Command::new("xclip");
        cmd.arg("-selection").arg("clipboard").arg("-in");
        cmd.stdin(std::process::Stdio::piped());
        let mut child = cmd
            .spawn()
            .context("Failed to spawn xclip. Please install wl-copy (Wayland) or xclip (X11).")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait().context("Failed to copy to clipboard")?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        anyhow::bail!("Clipboard not supported on this platform")
    }
}

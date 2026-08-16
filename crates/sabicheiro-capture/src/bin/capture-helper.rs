#[cfg(target_os = "windows")]
use sabicheiro_capture::HelperNonce;
#[cfg(target_os = "windows")]
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "windows"))]
    return Err("the capture helper is only available on Windows".into());

    #[cfg(target_os = "windows")]
    {
        let mut arguments = env::args().skip(1);
        let mut serve = false;
        let mut client_pid = None;
        let mut nonce = None;
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--serve" => serve = true,
                "--client-pid" => {
                    client_pid = arguments.next().and_then(|value| value.parse().ok())
                }
                "--nonce" => nonce = arguments.next().and_then(|value| parse_nonce(&value)),
                "--help" | "-h" => {
                    println!("capture-helper --serve --client-pid PID --nonce 64_HEX_CHARACTERS");
                    return Ok(());
                }
                _ => return Err("unsupported helper argument".into()),
            }
        }
        if !serve {
            return Err("the helper only supports the closed --serve operation".into());
        }
        let client_pid = client_pid
            .filter(|pid| *pid > 0)
            .ok_or("missing client PID")?;
        let nonce = nonce.ok_or("missing or invalid session nonce")?;
        sabicheiro_capture::run_windows_helper(nonce, client_pid)?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn parse_nonce(value: &str) -> Option<HelperNonce> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut nonce = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        nonce[index] = u8::from_str_radix(text, 16).ok()?;
    }
    let nonce = HelperNonce(nonce);
    (!nonce.is_zero()).then_some(nonce)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn nonce_parser_is_exact_and_rejects_zero() {
        assert_eq!(parse_nonce(&"5a".repeat(32)), Some(HelperNonce([0x5a; 32])));
        assert!(parse_nonce(&"00".repeat(32)).is_none());
        assert!(parse_nonce("5a").is_none());
        assert!(parse_nonce(&"zz".repeat(32)).is_none());
    }
}

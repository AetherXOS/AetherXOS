pub const MOK_SUBJECT: &str = "/CN=Aether_X_OS_Local_Platform_Key/";

pub fn openssl_mok_args(key_path: &str, cert_path: &str) -> Vec<String> {
    [
        "req",
        "-new",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-keyout",
        key_path,
        "-out",
        cert_path,
        "-days",
        "3650",
        "-nodes",
        "-subj",
        MOK_SUBJECT,
    ]
    .into_iter()
    .map(|part| part.to_string())
    .collect()
}

pub fn sbsign_args(key_path: &str, cert_path: &str, kernel_path: &str) -> Vec<String> {
    ["--key", key_path, "--cert", cert_path, "--output", kernel_path, kernel_path]
        .into_iter()
        .map(|part| part.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MOK_SUBJECT, openssl_mok_args, sbsign_args};

    #[test]
    fn openssl_args_embed_subject_and_paths() {
        let args = openssl_mok_args("k.key", "c.crt");
        assert!(args.windows(2).any(|w| w == ["-keyout", "k.key"]));
        assert!(args.windows(2).any(|w| w == ["-out", "c.crt"]));
        assert!(args.iter().any(|v| v == MOK_SUBJECT));
    }

    #[test]
    fn sbsign_args_place_kernel_as_input_and_output() {
        let args = sbsign_args("k.key", "c.crt", "kernel.efi");
        assert!(args.windows(2).any(|w| w == ["--key", "k.key"]));
        assert!(args.windows(2).any(|w| w == ["--cert", "c.crt"]));
        assert!(args.windows(2).any(|w| w == ["--output", "kernel.efi"]));
        assert_eq!(args.last().map(|s| s.as_str()), Some("kernel.efi"));
    }
}

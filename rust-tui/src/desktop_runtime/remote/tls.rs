use rcgen::generate_simple_self_signed;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::ServerConfig;

pub(super) fn load_or_create_tls(root: &Path) -> io::Result<(Arc<ServerConfig>, String)> {
    let cert_path = root.join("tls-cert.der");
    let key_path = root.join("tls-key.der");
    let existing = match (fs::read(&cert_path), fs::read(&key_path)) {
        (Ok(cert), Ok(key)) => (cert, key),
        (Err(cert_error), _) if cert_error.kind() != io::ErrorKind::NotFound => {
            return Err(cert_error);
        }
        (_, Err(key_error)) if key_error.kind() != io::ErrorKind::NotFound => {
            return Err(key_error);
        }
        _ => generate_and_store_tls_pair(root, &cert_path, &key_path)?,
    };
    crate::paths::base::harden_private_tree(root)?;
    let (cert_der, config) = match build_tls_config(existing.0.clone(), existing.1) {
        Ok(config) => (existing.0, config),
        Err(_) => {
            let regenerated = generate_and_store_tls_pair(root, &cert_path, &key_path)?;
            let config = build_tls_config(regenerated.0.clone(), regenerated.1)?;
            (regenerated.0, config)
        }
    };
    let fingerprint = hex(&Sha256::digest(&cert_der));
    Ok((Arc::new(config), fingerprint))
}

fn build_tls_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> io::Result<ServerConfig> {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
    let provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_no_client_auth()
        .with_single_cert(vec![CertificateDer::from(cert_der)], key)
        .map_err(io::Error::other)
}

fn generate_and_store_tls_pair(
    root: &Path,
    cert_path: &Path,
    key_path: &Path,
) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let generated =
        generate_simple_self_signed(vec!["pad.local".to_string()]).map_err(io::Error::other)?;
    let cert = generated.cert.der().to_vec();
    let key = generated.key_pair.serialize_der();
    let suffix = super::random_id(8);
    let cert_temporary = root.join(format!(".tls-cert-{suffix}.tmp"));
    let key_temporary = root.join(format!(".tls-key-{suffix}.tmp"));
    write_private_new(&cert_temporary, &cert)?;
    if let Err(error) = write_private_new(&key_temporary, &key) {
        let _ = fs::remove_file(&cert_temporary);
        return Err(error);
    }
    if let Err(error) = replace_private_file(&cert_temporary, cert_path) {
        let _ = fs::remove_file(&cert_temporary);
        let _ = fs::remove_file(&key_temporary);
        return Err(error);
    }
    if let Err(error) = replace_private_file(&key_temporary, key_path) {
        let _ = fs::remove_file(&key_temporary);
        return Err(error);
    }
    crate::paths::base::harden_private_tree(root)?;
    Ok((cert, key))
}

fn replace_private_file(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::remove_file(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::rename(source, destination)
}

fn write_private_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

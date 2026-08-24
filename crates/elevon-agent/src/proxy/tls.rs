use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use elevon_contracts::deploy::{TlsType, resolve_app_env_name};
use elevon_fs::agent::write_tls_file;
use pingora::protocols::tls::TlsRef;
use pingora::tls::pkey::{PKey, Private};
use pingora::tls::ssl::NameType;
use pingora::tls::x509::X509;

#[derive(Clone)]
pub struct DynamicCert {
    #[allow(clippy::type_complexity)]
    pub certs: Arc<ArcSwap<HashMap<String, (X509, PKey<Private>)>>>,
}

impl DynamicCert {
    pub fn new() -> Self {
        Self {
            certs: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
        }
    }

    fn get_stored_key_paths(
        &self,
        project: &str,
        app_name: &str,
        is_project: Option<bool>,
    ) -> Result<(PathBuf, PathBuf)> {
        let env_path = resolve_app_env_name(project, app_name);
        let stored_cert = elevon_fs::agent::get_tls_file(&env_path, TlsType::Cert, is_project)?;
        let stored_key = elevon_fs::agent::get_tls_file(&env_path, TlsType::Key, is_project)?;
        Ok((stored_cert, stored_key))
    }

    pub fn add_cert(
        &self,
        project: &str,
        app_name: &str,
        domain: String,
        is_project: Option<bool>,
    ) -> Result<()> {
        let (stored_cert, stored_key) = self.get_stored_key_paths(project, app_name, is_project)?;
        let cert_bytes = std::fs::read(stored_cert)?;
        let key_bytes = std::fs::read(stored_key)?;

        let cert = X509::from_pem(&cert_bytes)?;
        let key = PKey::private_key_from_pem(&key_bytes)?;

        self.certs.rcu(|c| {
            let mut next = c.as_ref().clone();
            next.insert(domain.clone(), (cert.clone(), key.clone()));
            next
        });

        Ok(())
    }

    fn find_certs_with_hostname(&self, hostname: &str) -> Option<(Arc<X509>, Arc<PKey<Private>>)> {
        self.certs
            .load()
            .get(hostname)
            .map(|(c, k)| (Arc::new(c.clone()), Arc::new(k.clone())))
    }

    pub fn setup_agent_certs(
        &self,
        domain: &str,
        cert_path: Option<&str>,
        key_path: Option<&str>,
    ) -> Result<()> {
        if let (Some(cert), Some(key)) = (cert_path, key_path) {
            let cert_bytes = std::fs::read(cert)?;
            let key_bytes = std::fs::read(key)?;
            write_tls_file("agent", &cert_bytes, TlsType::Cert, None)?;
            write_tls_file("agent", &key_bytes, TlsType::Key, None)?;
        }

        // TODO: Fix this. This is not ideal but for now its ok
        self.add_cert("agent", "agent", domain.to_string(), None)?;

        Ok(())
    }
}

#[async_trait]
impl pingora::listeners::TlsAccept for DynamicCert {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        use pingora::tls::ext;

        if self.certs.load().is_empty() {
            tracing::warn!("No certificates are configured.");
            return;
        }

        if let Some(server_name) = ssl.servername(NameType::HOST_NAME)
            && let Some((cert, key)) = self.find_certs_with_hostname(server_name)
        {
            ext::ssl_use_certificate(ssl, &cert).unwrap();
            ext::ssl_use_private_key(ssl, &key).unwrap();
            return;
        }
    }
}

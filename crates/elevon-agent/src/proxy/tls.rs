use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use elevon_contracts::deploy::TlsType;
use elevon_fs::agent::TlsOptions;
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
        agent_name: &str,
        project: &str,
        app: &str,
    ) -> Result<(PathBuf, PathBuf)> {
        let tls_options = TlsOptions {
            project: project.to_string(),
            app: Some(app.to_string()),
        };

        let stored_cert =
            elevon_fs::agent::get_tls_file(agent_name, TlsType::Cert, tls_options.clone())?;
        let stored_key = elevon_fs::agent::get_tls_file(agent_name, TlsType::Key, tls_options)?;

        Ok((stored_cert, stored_key))
    }

    fn add_cert_from_paths(&self, domain: String, cert_path: &str, key_path: &str) -> Result<()> {
        let cert_bytes = std::fs::read(cert_path)?;
        let key_bytes = std::fs::read(key_path)?;

        let cert = X509::from_pem(&cert_bytes)?;
        let key = PKey::private_key_from_pem(&key_bytes)?;

        self.certs.rcu(|current| {
            let mut next = current.as_ref().clone();
            next.insert(domain.clone(), (cert.clone(), key.clone()));
            next
        });

        Ok(())
    }

    pub fn add_cert(
        &self,
        agent_name: &str,
        project: &str,
        app: &str,
        domain: String,
    ) -> Result<()> {
        let (stored_cert, stored_key) = self.get_stored_key_paths(agent_name, project, app)?;

        self.add_cert_from_paths(
            domain,
            stored_cert.to_str().context("invalid certificate path")?,
            stored_key.to_str().context("invalid key path")?,
        )
    }

    fn find_certs_with_hostname(&self, hostname: &str) -> Option<(Arc<X509>, Arc<PKey<Private>>)> {
        self.certs
            .load()
            .get(hostname)
            .map(|(c, k)| (Arc::new(c.clone()), Arc::new(k.clone())))
    }

    pub fn setup_agent_certs(&self, agent_name: &str, domain: &str) -> Result<()> {
        let tls_options = TlsOptions {
            project: "agent".to_string(),
            app: None,
        };

        let stored_cert =
            elevon_fs::agent::get_tls_file(agent_name, TlsType::Cert, tls_options.clone())?;
        let stored_key = elevon_fs::agent::get_tls_file(agent_name, TlsType::Key, tls_options)?;

        self.add_cert_from_paths(
            domain.to_string(),
            stored_cert.to_str().context("invalid certificate path")?,
            stored_key.to_str().context("invalid key path")?,
        )?;

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

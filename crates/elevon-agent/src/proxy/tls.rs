use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use pingora::protocols::tls::TlsRef;
use pingora::tls::pkey::{PKey, Private};
use pingora::tls::ssl::NameType;
use pingora::tls::x509::X509;

#[derive(Clone)]
pub struct DynamicCert {
    pub certs: Arc<ArcSwap<HashMap<String, (X509, PKey<Private>)>>>,
}

impl DynamicCert {
    pub fn new() -> Self {
        Self {
            certs: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
        }
    }

    pub fn add_cert(&self, domain: String, cert_path: &str, key_path: &str) -> Result<()> {
        let cert_bytes = std::fs::read(cert_path)?;
        let cert = X509::from_pem(&cert_bytes)?;

        let key_bytes = std::fs::read(key_path)?;
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
}

#[async_trait]
impl pingora::listeners::TlsAccept for DynamicCert {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        use pingora::tls::ext;

        if self.certs.load().is_empty() {
            tracing::warn!("No certificates are configured.");
            return;
        }

        if let Some(server_name) = ssl.servername(NameType::HOST_NAME) {
            if let Some((cert, key)) = self.find_certs_with_hostname(server_name) {
                ext::ssl_use_certificate(ssl, &cert).unwrap();
                ext::ssl_use_private_key(ssl, &key).unwrap();
                return;
            }
        }
    }
}

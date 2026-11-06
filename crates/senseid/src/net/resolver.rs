//! A `reqwest` DNS resolver that hands the connector only public addresses
//! (tracker spec §G5.3).
//!
//! Every resolution is filtered, so every new connection is re-checked (DNS rebinding
//! included). An IP-literal URL never reaches a resolver; callers refuse those
//! separately (`tracker::http::normalize_base_url`).

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// Turns a host name into addresses. The seam a test replaces.
#[async_trait::async_trait]
pub trait Lookup: Send + Sync {
    async fn lookup(&self, host: &str) -> std::io::Result<Vec<IpAddr>>;
}

/// The operating system's resolver.
pub struct SystemLookup;

#[async_trait::async_trait]
impl Lookup for SystemLookup {
    async fn lookup(&self, host: &str) -> std::io::Result<Vec<IpAddr>> {
        Ok(tokio::net::lookup_host((host, 0)).await?.map(|a| a.ip()).collect())
    }
}

/// Why a name produced no address the client may use.
#[derive(Debug)]
pub enum ResolveError {
    Lookup(std::io::Error),
    /// Every address the name resolved to was refused by the address rule.
    Refused {
        host: String,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::Lookup(e) => write!(f, "lookup failed: {e}"),
            ResolveError::Refused { host } => write!(f, "{host}: no public address"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Resolves through `lookup` and keeps only the addresses `allowed` accepts.
pub struct PublicOnly {
    lookup: Arc<dyn Lookup>,
    allowed: fn(IpAddr) -> bool,
}

impl PublicOnly {
    /// The production resolver: the system lookup and [`crate::net::addr::is_public_ip`].
    pub fn system() -> Self {
        Self::new(Arc::new(SystemLookup), crate::net::addr::is_public_ip)
    }

    pub fn new(lookup: Arc<dyn Lookup>, allowed: fn(IpAddr) -> bool) -> Self {
        PublicOnly { lookup, allowed }
    }

    /// The addresses of `host` the client may connect to; never empty on `Ok`.
    pub async fn resolve_checked(&self, host: &str) -> Result<Vec<IpAddr>, ResolveError> {
        let addrs = self.lookup.lookup(host).await.map_err(ResolveError::Lookup)?;
        let kept = keep_allowed(addrs, self.allowed);
        if kept.is_empty() {
            return Err(ResolveError::Refused { host: host.to_string() });
        }
        Ok(kept)
    }
}

/// Pure: the addresses `allowed` accepts, in their original order.
fn keep_allowed(addrs: Vec<IpAddr>, allowed: fn(IpAddr) -> bool) -> Vec<IpAddr> {
    addrs.into_iter().filter(|ip| allowed(*ip)).collect()
}

impl reqwest::dns::Resolve for PublicOnly {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let this = PublicOnly { lookup: Arc::clone(&self.lookup), allowed: self.allowed };
        Box::pin(async move {
            let ips = this.resolve_checked(name.as_str()).await?;
            let addrs: reqwest::dns::Addrs =
                Box::new(ips.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Answers every name with fixed addresses, and records which names it was asked.
    #[derive(Default)]
    pub(crate) struct StubLookup {
        pub answer: Vec<IpAddr>,
        pub asked: std::sync::Mutex<Vec<String>>,
    }

    impl StubLookup {
        pub(crate) fn answering(ips: &[&str]) -> Arc<Self> {
            Arc::new(StubLookup {
                answer: ips.iter().map(|ip| ip.parse().unwrap()).collect(),
                asked: Default::default(),
            })
        }
    }

    #[async_trait::async_trait]
    impl Lookup for StubLookup {
        async fn lookup(&self, host: &str) -> std::io::Result<Vec<IpAddr>> {
            self.asked.lock().unwrap().push(host.to_string());
            Ok(self.answer.clone())
        }
    }

    fn production_rule(stub: Arc<StubLookup>) -> PublicOnly {
        PublicOnly::new(stub, crate::net::addr::is_public_ip)
    }

    #[tokio::test]
    async fn every_refused_address_is_a_distinct_refusal() {
        for ip in [
            "10.0.0.5",
            "127.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "169.254.169.254",
            "100.64.0.1",
            "0.1.2.3",
            "64:ff9b::a00:1",
            "fd00:ec2::254",
            "fd12::1",
            "100.100.100.200",
            "2002:7f00:1::",
        ] {
            let r = production_rule(StubLookup::answering(&[ip]))
                .resolve_checked("gitlab.example.test")
                .await;
            assert!(matches!(r, Err(ResolveError::Refused { .. })), "{ip} must be refused: {r:?}");
        }
    }

    #[tokio::test]
    async fn a_public_address_reaches_the_caller() {
        let r = production_rule(StubLookup::answering(&["93.184.216.34"]))
            .resolve_checked("gitlab.example.test")
            .await
            .unwrap();
        assert_eq!(r, vec!["93.184.216.34".parse::<IpAddr>().unwrap()]);
    }

    #[tokio::test]
    async fn only_the_public_addresses_of_a_mixed_answer_are_kept() {
        let r = production_rule(StubLookup::answering(&["10.0.0.5", "1.1.1.1", "::1"]))
            .resolve_checked("gitlab.example.test")
            .await
            .unwrap();
        assert_eq!(r, vec!["1.1.1.1".parse::<IpAddr>().unwrap()]);
    }

    #[tokio::test]
    async fn the_reqwest_resolve_impl_applies_the_rule() {
        use reqwest::dns::Resolve;
        let r = production_rule(StubLookup::answering(&["169.254.169.254"]))
            .resolve("gitlab.example.test".parse().unwrap())
            .await;
        let err = r.err().expect("refused");
        assert!(err.downcast_ref::<ResolveError>().is_some(), "{err}");
    }
}

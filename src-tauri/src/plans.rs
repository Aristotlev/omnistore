use crate::model::{PlanMatch, Provider};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tier {
    pub id: String,
    pub name: String,
    pub gb: f64,
    pub monthly_eur: f64,
    pub verified: bool,
    #[serde(default)]
    pub bundle: bool,
}

impl Tier {
    pub fn bytes(&self) -> u64 {
        (self.gb * GIB) as u64
    }
    fn to_match(&self, exact: bool) -> PlanMatch {
        PlanMatch {
            id: self.id.clone(),
            name: self.name.clone(),
            bytes: self.bytes(),
            monthly_eur: self.monthly_eur,
            price_verified: self.verified,
            exact,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogue {
    pub name: String,
    pub manage_url: String,
    pub tiers: Vec<Tier>,
}

#[derive(Debug, Deserialize)]
pub struct Catalogue {
    pub providers: HashMap<String, ProviderCatalogue>,
}

static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();

pub fn catalogue() -> &'static Catalogue {
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!("../resources/plans.json")).expect("plans.json is valid")
    })
}

pub fn provider(p: Provider) -> &'static ProviderCatalogue {
    let key = match p {
        Provider::Icloud => "icloud",
        Provider::Google => "google",
    };
    &catalogue().providers[key]
}

/// Storage-only tiers (bundles like AI Pro excluded) sorted by size then price.
fn storage_tiers(p: Provider) -> Vec<&'static Tier> {
    let mut t: Vec<&Tier> = provider(p).tiers.iter().filter(|t| !t.bundle).collect();
    t.sort_by(|a, b| a.gb.partial_cmp(&b.gb).unwrap().then(a.monthly_eur.partial_cmp(&b.monthly_eur).unwrap()));
    t
}

/// Infer the plan from the quota size. Providers report quotas in binary units with small
/// differences, so pick the tier whose size is closest on a log scale.
pub fn match_by_quota(p: Provider, quota_bytes: u64) -> PlanMatch {
    let tiers = storage_tiers(p);
    let q = (quota_bytes.max(1)) as f64;
    let best = tiers
        .iter()
        .min_by(|a, b| {
            let da = (q / a.bytes() as f64).ln().abs();
            let db = (q / b.bytes() as f64).ln().abs();
            da.partial_cmp(&db).unwrap()
        })
        .expect("catalogue has tiers");
    let mut m = best.to_match(false);
    // Family / custom quotas that do not land near a tier keep the real size.
    let drift = (q / best.bytes() as f64).ln().abs();
    if drift > 0.25 {
        m.name = format!("{} (custom quota {:.0} GB)", m.name, q / GIB);
        m.bytes = quota_bytes;
    }
    m
}

/// Cheapest storage tier that holds `need_bytes` with `headroom` (e.g. 1.15 = 15%).
pub fn cheapest_fit(p: Provider, need_bytes: u64, headroom: f64) -> Option<PlanMatch> {
    let need = (need_bytes as f64 * headroom) as u64;
    storage_tiers(p)
        .into_iter()
        .filter(|t| t.bytes() >= need)
        .min_by(|a, b| a.monthly_eur.partial_cmp(&b.monthly_eur).unwrap())
        .map(|t| t.to_match(false))
}

pub fn manage_url(p: Provider) -> String {
    provider(p).manage_url.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_google_free() {
        assert_eq!(match_by_quota(Provider::Google, 16_106_127_360).id, "google-free");
    }
    #[test]
    fn matches_icloud_200() {
        assert_eq!(match_by_quota(Provider::Icloud, 214_748_364_800).id, "icloud-200");
    }
    #[test]
    fn cheapest_fit_works() {
        let gib = 1024u64.pow(3);
        assert_eq!(cheapest_fit(Provider::Icloud, 120 * gib, 1.15).unwrap().id, "icloud-200");
        assert_eq!(cheapest_fit(Provider::Google, 10 * gib, 1.15).unwrap().id, "google-free");
    }
}

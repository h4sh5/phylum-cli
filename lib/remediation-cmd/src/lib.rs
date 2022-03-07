use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "package_manager")]
pub enum Remediation {
    Yarn(yarn::Remediation),
}

mod yarn {
    use node_semver::{Range, Version};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RangeSpec {
        name: String,
        range_key: String,
        range: Range,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "action")]
    pub enum Remediation {
        Downgrade { spec: RangeSpec, version: Version },
        Upgrade { spec: RangeSpec, version: Version },
        NoViableCandidate { spec: RangeSpec },
    }
}

#[async_trait]
pub trait PhylumApiExt {
    async fn remediation_yarn<'a>(&self, lockfile: &'a str, manifest: &'a str) -> Result<()>;
}

#[async_trait]
impl PhylumApiExt for phylum_cli::api::PhylumApi {
    async fn remediation_yarn<'a>(&self, lockfile: &'a str, manifest: &'a str) -> Result<()> {
        let remediations = self
            .post::<Remediation, _>(
                self.route("/data/packages/remediation/yarn"),
                serde_json::json!({
                    "manifest": manifest,
                    "lockfile": lockfile,
                }),
            )
            .await?;

        println!("{:?}", remediations);

        Ok(())
    }
}

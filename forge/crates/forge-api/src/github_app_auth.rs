use std::time::{SystemTime, UNIX_EPOCH};

use forge_types::ForgeError;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct GitHubAppConfig {
    pub app_id: String,
    pub private_key_pem: String,
    pub api_base_url: String,
}

impl GitHubAppConfig {
    pub fn from_env() -> Result<Self, ForgeError> {
        let app_id = std::env::var("GITHUB_APP_ID")
            .map_err(|_| ForgeError::Config("GITHUB_APP_ID is required".into()))?;
        let private_key_pem = match std::env::var("GITHUB_APP_PRIVATE_KEY") {
            Ok(value) => value.replace("\\n", "\n"),
            Err(_) => {
                let path = std::env::var("GITHUB_APP_PRIVATE_KEY_PATH").map_err(|_| {
                    ForgeError::Config(
                        "GITHUB_APP_PRIVATE_KEY or GITHUB_APP_PRIVATE_KEY_PATH is required".into(),
                    )
                })?;
                std::fs::read_to_string(path).map_err(ForgeError::Io)?
            }
        };
        let api_base_url = std::env::var("GITHUB_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.github.com".to_string());

        Ok(Self {
            app_id,
            private_key_pem,
            api_base_url,
        })
    }
}

#[derive(Debug, Clone)]
pub struct GitHubAppClient {
    config: GitHubAppConfig,
    client: reqwest::Client,
}

impl GitHubAppClient {
    pub fn new(config: GitHubAppConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn installation_token(&self, installation_id: u64) -> Result<String, ForgeError> {
        let jwt = self.app_jwt()?;
        let url = format!(
            "{}/app/installations/{installation_id}/access_tokens",
            self.config.api_base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(url)
            .header(USER_AGENT, "Forge")
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .bearer_auth(jwt)
            .send()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ForgeError::Http(format!(
                "GitHub installation token request failed {status}: {body}"
            )));
        }

        let token = response
            .json::<InstallationTokenResponse>()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;
        Ok(token.token)
    }

    pub async fn post_issue_comment(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<GitHubCommentResponse, ForgeError> {
        let token = self.installation_token(installation_id).await?;
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{issue_number}/comments",
            self.config.api_base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(url)
            .header(USER_AGENT, "Forge")
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .bearer_auth(token)
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ForgeError::Http(format!(
                "GitHub issue comment request failed {status}: {body}"
            )));
        }

        response
            .json::<GitHubCommentResponse>()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))
    }

    fn app_jwt(&self) -> Result<String, ForgeError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ForgeError::Config(format!("system time error: {e}")))?
            .as_secs() as usize;
        let claims = GitHubAppJwtClaims {
            iat: now.saturating_sub(60),
            exp: now + 9 * 60,
            iss: self.config.app_id.clone(),
        };
        let key = EncodingKey::from_rsa_pem(self.config.private_key_pem.as_bytes())
            .map_err(|e| ForgeError::Config(format!("invalid GitHub App private key: {e}")))?;
        encode(&Header::new(Algorithm::RS256), &claims, &key)
            .map_err(|e| ForgeError::Config(format!("failed to sign GitHub App JWT: {e}")))
    }
}

#[derive(Debug, Serialize)]
struct GitHubAppJwtClaims {
    iat: usize,
    exp: usize,
    iss: String,
}

#[derive(Debug, Deserialize)]
struct InstallationTokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubCommentResponse {
    pub id: u64,
}

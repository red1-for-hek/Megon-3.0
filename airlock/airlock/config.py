"""Configuration loaded from environment / .env."""

from __future__ import annotations

import os
import ssl
from dataclasses import dataclass

from dotenv import load_dotenv


def _ensure_ca_bundle() -> None:
    """Point Python at certifi's CA bundle when the platform default is missing.

    The python.org macOS builds ship no CA bundle (their default `openssl_cafile` doesn't
    exist), so every HTTPS call — Daytona, ai&, Oxylabs — fails with CERTIFICATE_VERIFY_FAILED
    until you run "Install Certificates.command". This makes the gate (and the hook subprocess)
    work out of the box. Conservative: only fills in vars that aren't already set, and only
    when the platform bundle is genuinely absent.
    """
    if os.environ.get("SSL_CERT_FILE") and os.environ.get("REQUESTS_CA_BUNDLE"):
        return
    default = ssl.get_default_verify_paths().openssl_cafile
    if default and os.path.exists(default):
        return
    try:
        import certifi
    except ImportError:
        return
    bundle = certifi.where()
    os.environ.setdefault("SSL_CERT_FILE", bundle)
    os.environ.setdefault("REQUESTS_CA_BUNDLE", bundle)


_ensure_ca_bundle()
load_dotenv()


@dataclass
class Config:
    daytona_api_key: str | None
    daytona_api_url: str | None
    aiand_api_key: str | None       # ai& — the judge (and static-read fallback)
    aiand_base_url: str
    aiand_model: str
    nosana_endpoint: str | None
    nosana_api_key: str
    nosana_model: str
    kimi_api_key: str | None         # Kimi — alternate static-read backend (flag value "k")
    kimi_base_url: str
    kimi_model: str
    oxylabs_username: str | None     # Oxylabs — reputation / web intel (optional)
    oxylabs_password: str | None
    doubleword_api_key: str | None   # Doubleword — embeddings similarity match (MATCH it)
    doubleword_base_url: str
    doubleword_embed_model: str


def load_config() -> Config:
    return Config(
        daytona_api_key=os.getenv("DAYTONA_API_KEY") or None,
        daytona_api_url=os.getenv("DAYTONA_API_URL") or None,
        aiand_api_key=os.getenv("AIAND_API_KEY") or None,
        aiand_base_url=os.getenv("AIAND_BASE_URL", "https://api.aiand.com/v1"),
        # List what your key can call with `client.models.list()`; deepseek-v4-flash is
        # cheap/fast with a 1M context (good under fan-out concurrency).
        aiand_model=os.getenv("AIAND_MODEL", "deepseek-v4-flash"),
        nosana_endpoint=os.getenv("NOSANA_ENDPOINT") or None,
        nosana_api_key=os.getenv("NOSANA_API_KEY") or "sk-no-key-required",
        nosana_model=os.getenv("NOSANA_MODEL", "Qwen/Qwen2.5-Coder-7B-Instruct"),
        kimi_api_key=os.getenv("KIMI_API_KEY") or None,
        kimi_base_url=os.getenv("KIMI_BASE_URL", "https://api.moonshot.ai/v1"),
        kimi_model=os.getenv("KIMI_MODEL", "kimi-k2-turbo-preview"),
        oxylabs_username=os.getenv("OXYLABS_USERNAME") or None,
        oxylabs_password=os.getenv("OXYLABS_PASSWORD") or None,
        doubleword_api_key=os.getenv("DOUBLEWORD_API_KEY") or None,
        doubleword_base_url=os.getenv("DOUBLEWORD_BASE_URL", "https://api.doubleword.ai/v1"),
        doubleword_embed_model=os.getenv("DOUBLEWORD_EMBED_MODEL", "Qwen/Qwen3-Embedding-8B"),
    )

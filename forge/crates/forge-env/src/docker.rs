use std::path::Path;

use bollard::container::{
    Config, CreateContainerOptions, DownloadFromContainerOptions, RemoveContainerOptions,
    StartContainerOptions, UploadToContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use forge_types::ForgeError;
use futures_util::TryStreamExt;

/// A running Docker container managed by forge-env.
pub struct DockerContainer {
    pub docker: Docker,
    pub container_id: String,
    pub image: String,
}

impl DockerContainer {
    /// Connect to Docker and create + start a container from the given image.
    ///
    /// `name` is an optional container name for identification.
    /// `env_vars` are environment variables to set in the container.
    /// `startup_commands` are commands to pass as `Cmd` (use a shell if you need multiple).
    pub async fn create_and_start(
        image: &str,
        name: Option<&str>,
        env_vars: &[(String, String)],
        startup_commands: &[String],
    ) -> Result<Self, ForgeError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ForgeError::Docker(e.to_string()))?;

        // Format env vars as KEY=VALUE strings.
        let env: Vec<String> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Build the Cmd list — fall back to an infinite sleep so the container
        // stays alive when no startup command is provided.
        let cmd: Vec<&str> = if startup_commands.is_empty() {
            vec!["/bin/sh", "-c", "tail -f /dev/null"]
        } else {
            startup_commands.iter().map(|s| s.as_str()).collect()
        };

        let create_opts = if let Some(n) = name {
            Some(CreateContainerOptions {
                name: n,
                platform: None,
            })
        } else {
            None
        };

        let env_refs: Vec<&str> = env.iter().map(|s| s.as_str()).collect();

        let config = Config {
            image: Some(image),
            cmd: Some(cmd),
            env: Some(env_refs),
            tty: Some(true),
            open_stdin: Some(true),
            stdin_once: Some(false),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            host_config: Some(HostConfig::default()),
            ..Default::default()
        };

        let response = docker
            .create_container(create_opts, config)
            .await
            .map_err(|e| ForgeError::Docker(format!("create_container: {}", e)))?;

        let container_id = response.id;

        docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| ForgeError::Docker(format!("start_container: {}", e)))?;

        Ok(Self {
            docker,
            container_id,
            image: image.to_string(),
        })
    }

    /// Copy a file or directory into the container at `dest_path`.
    ///
    /// The `dest_path` must be an absolute path to a *directory* inside the
    /// container; the archive is extracted there.
    pub async fn copy_into(&self, local_path: &Path, dest_path: &str) -> Result<(), ForgeError> {
        let tar_bytes = build_tar(local_path)?;

        let opts = UploadToContainerOptions {
            path: dest_path,
            no_overwrite_dir_non_dir: "",
        };

        self.docker
            .upload_to_container(&self.container_id, Some(opts), tar_bytes.into())
            .await
            .map_err(|e| ForgeError::Docker(format!("upload_to_container: {}", e)))?;

        Ok(())
    }

    /// Copy a file out of the container at `container_path`.
    ///
    /// Returns the raw tar bytes (the Docker API wraps the file in a tar
    /// archive). Call `extract_tar_first_file` to get the raw content.
    pub async fn copy_out(&self, container_path: &str) -> Result<Vec<u8>, ForgeError> {
        let opts = DownloadFromContainerOptions { path: container_path };

        let stream = self
            .docker
            .download_from_container(&self.container_id, Some(opts));

        // Collect all chunks.
        let bytes: Vec<u8> = stream
            .try_fold(Vec::new(), |mut acc, chunk| async move {
                acc.extend_from_slice(&chunk);
                Ok(acc)
            })
            .await
            .map_err(|e| ForgeError::Docker(format!("download_from_container: {}", e)))?;

        Ok(bytes)
    }

    /// Stop and remove the container.
    pub async fn remove(self) -> Result<(), ForgeError> {
        // Best-effort stop — ignore errors (container may already be stopped).
        let _ = self
            .docker
            .stop_container(&self.container_id, None)
            .await;

        self.docker
            .remove_container(
                &self.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| ForgeError::Docker(format!("remove_container: {}", e)))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an in-memory tar archive containing `local_path`.
///
/// If `local_path` is a file the archive contains a single entry with the
/// file's base name. If it is a directory the archive contains all files
/// inside it (non-recursively for now — extend as needed).
fn build_tar(local_path: &Path) -> Result<Vec<u8>, ForgeError> {
    let buf = Vec::new();
    let mut builder = tar::Builder::new(buf);

    if local_path.is_dir() {
        builder
            .append_dir_all(".", local_path)
            .map_err(ForgeError::Io)?;
    } else {
        let file_name = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ForgeError::Environment(
                format!("invalid path for tar: {}", local_path.display())
            ))?;

        let mut f = std::fs::File::open(local_path).map_err(ForgeError::Io)?;
        builder
            .append_file(file_name, &mut f)
            .map_err(ForgeError::Io)?;
    }

    builder.finish().map_err(ForgeError::Io)?;
    Ok(builder.into_inner().map_err(ForgeError::Io)?)
}

/// Extract the content of the first file from a tar archive (as produced by
/// `docker cp`).
pub fn extract_tar_first_file(tar_bytes: &[u8]) -> Result<Vec<u8>, ForgeError> {
    let mut archive = tar::Archive::new(tar_bytes);
    let mut entries = archive
        .entries()
        .map_err(ForgeError::Io)?;

    if let Some(entry_result) = entries.next() {
        let mut entry = entry_result.map_err(ForgeError::Io)?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data).map_err(ForgeError::Io)?;
        return Ok(data);
    }

    Err(ForgeError::Environment(
        "tar archive from container is empty".to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `build_tar` produces a valid tar archive containing the
    /// expected file, and that `extract_tar_first_file` can recover it.
    #[test]
    fn tar_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, b"hello forge").expect("write");

        let tar_bytes = build_tar(&file_path).expect("build_tar");
        let recovered = extract_tar_first_file(&tar_bytes).expect("extract");
        assert_eq!(recovered, b"hello forge");
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn container_create_start_remove() {
        let container =
            DockerContainer::create_and_start("alpine:latest", None, &[], &[])
                .await
                .expect("create_and_start");

        assert!(!container.container_id.is_empty());
        container.remove().await.expect("remove");
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn container_copy_roundtrip() {
        let container =
            DockerContainer::create_and_start("alpine:latest", None, &[], &[])
                .await
                .expect("create_and_start");

        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"forge test content").expect("write");

        container
            .copy_into(&file_path, "/tmp")
            .await
            .expect("copy_into");

        let tar_bytes = container.copy_out("/tmp/test.txt").await.expect("copy_out");
        let content = extract_tar_first_file(&tar_bytes).expect("extract");
        assert_eq!(content, b"forge test content");

        container.remove().await.expect("remove");
    }
}

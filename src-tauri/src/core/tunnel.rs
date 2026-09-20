use std::sync::Arc;
use std::time::Duration;

use russh::client::{AuthResult, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::{client, Disconnect};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::core::error::DbError;
use crate::core::types::ConnectionInput;

struct AcceptAllHostKeys;

impl client::Handler for AcceptAllHostKeys {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SshTunnel {
    pub local_port: u16,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl std::fmt::Debug for SshTunnel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshTunnel")
            .field("local_port", &self.local_port)
            .finish()
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

impl SshTunnel {
    pub async fn open(input: &ConnectionInput) -> Result<Self, DbError> {
        let ssh_host = input.ssh_host.trim();
        let ssh_user = input.ssh_user.trim();
        if ssh_host.is_empty() || ssh_user.is_empty() {
            return Err(DbError::validation("SSH host and user are required"));
        }
        let ssh_port = if input.ssh_port == 0 { 22 } else { input.ssh_port };
        let dest_host = if input.host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            input.host.trim().to_string()
        };
        let dest_port = if input.port == 0 { 5432 } else { input.port };

        let mut config = client::Config::default();
        config.keepalive_interval = Some(Duration::from_secs(30));
        let config = Arc::new(config);

        let mut session = tokio::time::timeout(
            Duration::from_secs(15),
            client::connect(config, (ssh_host, ssh_port), AcceptAllHostKeys),
        )
        .await
        .map_err(|_| DbError::connection("SSH tunnel timed out while connecting"))?
        .map_err(|err| DbError::connection(format!("SSH tunnel failed: {err}")))?;

        authenticate_ssh(&mut session, input).await?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| DbError::connection(format!("Could not bind SSH local port: {err}")))?;
        let local_port = listener
            .local_addr()
            .map_err(|err| DbError::connection(format!("Could not read SSH local port: {err}")))?
            .port();

        let cancel = CancellationToken::new();
        let cancel_loop = cancel.clone();
        let handle = Arc::new(session);
        let forward_handle = handle.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_loop.cancelled() => break,
                    accepted = listener.accept() => {
                        let Ok((mut tcp, peer)) = accepted else { break; };
                        let channel_handle = forward_handle.clone();
                        let dest_host = dest_host.clone();
                        tokio::spawn(async move {
                            match channel_handle
                                .channel_open_direct_tcpip(
                                    dest_host,
                                    dest_port as u32,
                                    peer.ip().to_string(),
                                    u32::from(peer.port()),
                                )
                                .await
                            {
                                Ok(channel) => {
                                    let mut ssh: russh::ChannelStream<russh::client::Msg> =
                                        channel.into_stream();
                                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut ssh).await;
                                }
                                Err(err) => {
                                    tracing::warn!("SSH forward channel failed: {err}");
                                }
                            }
                        });
                    }
                }
            }
            let _ = handle
                .disconnect(Disconnect::ByApplication, "tunnel closed", "")
                .await;
        });

        Ok(Self {
            local_port,
            cancel,
            task,
        })
    }
}

async fn authenticate_ssh(
    session: &mut Handle<AcceptAllHostKeys>,
    input: &ConnectionInput,
) -> Result<(), DbError> {
    let user = input.ssh_user.trim();
    let mut last_error = None;

    let key_path = input.ssh_private_key_path.trim();
    if !key_path.is_empty() {
        let passphrase = if input.ssh_key_passphrase.is_empty() {
            None
        } else {
            Some(input.ssh_key_passphrase.as_str())
        };
        match load_secret_key(key_path, passphrase) {
            Ok(key) => {
                let hash = match session.best_supported_rsa_hash().await {
                    Ok(value) => value.flatten(),
                    Err(_) => None,
                };
                match session
                    .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                    .await
                {
                    Ok(result) if auth_ok(&result) => return Ok(()),
                    Ok(_) => last_error = Some("SSH key was rejected".to_string()),
                    Err(err) => last_error = Some(format!("SSH key auth failed: {err}")),
                }
            }
            Err(err) => last_error = Some(format!("Could not read SSH private key: {err}")),
        }
    }

    if !input.ssh_password.is_empty() {
        match session
            .authenticate_password(user, input.ssh_password.clone())
            .await
        {
            Ok(result) if auth_ok(&result) => return Ok(()),
            Ok(_) => last_error = Some("SSH password was rejected".to_string()),
            Err(err) => last_error = Some(format!("SSH password auth failed: {err}")),
        }
    }

    Err(DbError::auth(last_error.unwrap_or_else(|| {
        "SSH authentication failed. Provide a password or private key.".to_string()
    })))
}

fn auth_ok(result: &AuthResult) -> bool {
    result.success()
}

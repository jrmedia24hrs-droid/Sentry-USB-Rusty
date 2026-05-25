//! Handshake: SessionInfoRequest → SessionInfo, per Tesla domain.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message;
use rand::RngCore;
use tracing::{debug, info};

use crate::gatt::Connection;
use crate::keys::KeyPair;
use crate::proto::signatures::SessionInfo;
use crate::proto::universal_message::{
    Destination, Domain, RoutableMessage, SessionInfoRequest, destination,
    routable_message,
};

/// Result of one SessionInfoRequest exchange. The raw bytes match
/// what tesla-control writes into its session-cache JSON, so we
/// can mirror its on-disk format byte-for-byte.
#[derive(Debug, Clone)]
pub struct SessionInfoResponse {
    pub domain: Domain,
    /// Raw SessionInfo proto bytes — keep for cache-file compat.
    pub raw: Vec<u8>,
    pub parsed: SessionInfo,
}

/// Send SessionInfoRequest to `domain` and decode the response.
pub async fn request_session_info(
    conn: &mut Connection,
    keypair: &KeyPair,
    domain: Domain,
) -> Result<SessionInfoResponse> {
    let payload = build_request(keypair, domain);
    debug!(
        "session-info: TX {} bytes to {:?}",
        payload.len(),
        domain
    );
    let response = conn
        .round_trip(&payload, Duration::from_secs(10))
        .await
        .context("session-info round-trip")?;
    debug!("session-info: RX {} bytes", response.len());
    parse_response(&response, domain)
}

fn build_request(keypair: &KeyPair, domain: Domain) -> Vec<u8> {
    let from_uuid = random_uuid_bytes();
    let req_uuid = random_uuid_bytes();
    let msg = RoutableMessage {
        to_destination: Some(Destination {
            sub_destination: Some(destination::SubDestination::Domain(domain as i32)),
        }),
        from_destination: Some(Destination {
            sub_destination: Some(destination::SubDestination::RoutingAddress(
                from_uuid.to_vec(),
            )),
        }),
        payload: Some(routable_message::Payload::SessionInfoRequest(
            SessionInfoRequest {
                public_key: keypair.pub_uncompressed.clone(),
                challenge: Vec::new(),
            },
        )),
        uuid: req_uuid.to_vec(),
        ..Default::default()
    };
    msg.encode_to_vec()
}

fn parse_response(bytes: &[u8], domain: Domain) -> Result<SessionInfoResponse> {
    let routable =
        RoutableMessage::decode(bytes).context("decode outer Routable")?;
    let raw = match routable.payload {
        Some(routable_message::Payload::SessionInfo(b)) => b,
        Some(other) => bail!("expected session_info, got {:?}", other),
        None => bail!(
            "response has no payload (signedMessageStatus={:?})",
            routable.signed_message_status
        ),
    };
    let parsed =
        SessionInfo::decode(raw.as_slice()).context("decode SessionInfo proto")?;
    info!(
        "session-info from {:?}: counter={}, clock_time={}, pubkey={} bytes",
        domain,
        parsed.counter,
        parsed.clock_time,
        parsed.public_key.len()
    );
    Ok(SessionInfoResponse {
        domain,
        raw,
        parsed,
    })
}

fn random_uuid_bytes() -> [u8; 16] {
    let mut out = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut out);
    out
}

//! Decode a successful Infotainment state-query response into the
//! typed `car_server.VehicleData` sub-messages.
//!
//! The decrypted payload from a `state climate` (etc.) query is a
//! serialized `car_server.Response` proto. `Response.response_msg`
//! is a oneof; for `GetVehicleData` queries it's always
//! `VehicleData vehicleData = 2`. `VehicleData` then has per-state
//! sub-messages (charge_state, climate_state, drive_state, ...)
//! populated according to which sub-state was requested.

use anyhow::{Context, Result, bail};
use prost::Message;

use crate::proto::car_server::{
    ChargeState, ClimateState, ClosuresState, DriveState, LocationState,
    Response, TirePressureState, VehicleData, response,
};

/// Top-level: decode any state-query response and return the inner
/// `VehicleData` if the response was an OK vehicle-data variant.
pub fn parse_vehicle_data(payload: &[u8]) -> Result<VehicleData> {
    let resp = Response::decode(payload).context("decoding car_server.Response")?;
    // ActionStatus.result is OPERATIONSTATUS_OK (0) on success.
    if let Some(status) = resp.action_status.as_ref() {
        let reason = status
            .result_reason
            .as_ref()
            .and_then(|r| r.reason.as_ref())
            .map(|r| match r {
                crate::proto::car_server::result_reason::Reason::PlainText(s) => s.clone(),
            });
        // OperationStatus_E: OK=0, ERROR=1.
        if status.result != 0 {
            bail!(
                "car returned ActionStatus error (result={}): {:?}",
                status.result,
                reason
            );
        }
    }
    match resp.response_msg {
        Some(response::ResponseMsg::VehicleData(vd)) => Ok(vd),
        Some(other) => bail!("expected VehicleData response, got {:?}", other),
        None => bail!("response has no response_msg"),
    }
}

/// Decode a `state climate` response.
pub fn parse_climate(payload: &[u8]) -> Result<ClimateState> {
    parse_vehicle_data(payload)?
        .climate_state
        .context("response missing climate_state")
}

/// Decode a `state charge` response.
pub fn parse_charge(payload: &[u8]) -> Result<ChargeState> {
    parse_vehicle_data(payload)?
        .charge_state
        .context("response missing charge_state")
}

/// Decode a `state drive` response.
pub fn parse_drive(payload: &[u8]) -> Result<DriveState> {
    parse_vehicle_data(payload)?
        .drive_state
        .context("response missing drive_state")
}

/// Decode a `state location` response.
pub fn parse_location(payload: &[u8]) -> Result<LocationState> {
    parse_vehicle_data(payload)?
        .location_state
        .context("response missing location_state")
}

/// Decode a `state tire-pressure` response.
pub fn parse_tire_pressure(payload: &[u8]) -> Result<TirePressureState> {
    parse_vehicle_data(payload)?
        .tire_pressure_state
        .context("response missing tire_pressure_state")
}

/// Decode a `state closures` response.
pub fn parse_closures(payload: &[u8]) -> Result<ClosuresState> {
    parse_vehicle_data(payload)?
        .closures_state
        .context("response missing closures_state")
}

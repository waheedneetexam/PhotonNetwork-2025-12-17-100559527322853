use ic_cdk::api::caller;
use ic_cdk::api::management_canister::ecdsa::{
    ecdsa_public_key, EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgument,
};
use ic_cdk::api::management_canister::bitcoin::{
    bitcoin_get_utxos, 
    bitcoin_get_current_fee_percentiles, 
    BitcoinNetwork as IcpBitcoinNetwork, 
    GetUtxosRequest,
    GetCurrentFeePercentilesRequest,
    Utxo 
};
use bitcoin::{Address, Network, PublicKey};
use candid::{CandidType, Deserialize};
use serde::Serialize;

// --- DATA STRUCTURES ---
#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct AddressInfo {
    pub address: String,
    pub balance_sats: u64,
    pub utxo_count: u32, // <--- NEW FIELD: Returns the count (e.g., 3, 5)
    pub utxos: Vec<Utxo>,
}

// --- CONFIGURATION ---
fn get_key_id() -> EcdsaKeyId {
    EcdsaKeyId {
        curve: EcdsaCurve::Secp256k1,
        name: "test_key_1".to_string(), 
    }
}

fn get_network() -> Network {
    Network::Testnet 
}

fn get_icp_network() -> IcpBitcoinNetwork {
    IcpBitcoinNetwork::Testnet 
}

// --- HELPER: DERIVE ADDRESS ---
async fn derive_address_for_principal(p: candid::Principal) -> String {
    let (pk_response,) = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![p.as_slice().to_vec()],
        key_id: get_key_id(),
    }).await.expect("Failed to fetch public key");

    let public_key = PublicKey::from_slice(&pk_response.public_key)
        .expect("Invalid public key");
        
    Address::p2wpkh(&public_key, get_network())
        .expect("Failed to create address")
        .to_string()
}

// --- FUNCTION 1: GET ADDRESS ---
#[ic_cdk::update]
async fn get_btc_address() -> String {
    derive_address_for_principal(caller()).await
}

// --- FUNCTION 2: MASTER UTXO & BALANCE CHECKER ---
#[ic_cdk::update]
async fn get_utxos_and_balance(target_address: Option<String>) -> AddressInfo {
    // 1. Determine Address
    let address_to_check = match target_address {
        Some(addr) => addr.trim().to_string(),
        None => derive_address_for_principal(caller()).await,
    };

    // 2. Fetch UTXOs
    let (response,) = bitcoin_get_utxos(GetUtxosRequest {
        network: get_icp_network(), 
        address: address_to_check.clone(),
        filter: None, 
    })
    .await
    .expect("Failed to fetch UTXOs.");

    // 3. Calculate Totals
    let mut total_sats = 0;
    for utxo in &response.utxos {
        total_sats += utxo.value;
    }
    
    // Calculate Count
    let count = response.utxos.len() as u32;

    // 4. Return Data
    AddressInfo {
        address: address_to_check,
        balance_sats: total_sats,
        utxo_count: count, // Returns the number (e.g., 3)
        utxos: response.utxos,
    }
}

// --- NEW FUNCTION: GET ONLY THE COUNT ---
// Returns just the number (e.g., 5) for simpler logic checks
#[ic_cdk::update]
async fn get_utxo_count_only(target_address: Option<String>) -> u32 {
    let address_to_check = match target_address {
        Some(addr) => addr.trim().to_string(),
        None => derive_address_for_principal(caller()).await,
    };

    let (response,) = bitcoin_get_utxos(GetUtxosRequest {
        network: get_icp_network(), 
        address: address_to_check,
        filter: None, 
    })
    .await
    .expect("Failed to fetch UTXOs.");

    // Return just the length of the vector
    response.utxos.len() as u32
}

// --- DEBUG STATUS ---
#[ic_cdk::update]
async fn debug_network_status() -> String {
    let fees_result = bitcoin_get_current_fee_percentiles(
        GetCurrentFeePercentilesRequest { network: get_icp_network() }
    ).await;
    match fees_result {
        Ok(_) => "ICP Connection: ONLINE.".to_string(),
        Err(e) => format!("ICP Connection: OFFLINE. Error: {:?}", e),
    }
}

ic_cdk::export_candid!();
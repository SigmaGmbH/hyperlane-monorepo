use std::str::FromStr;

use derive_new::new;
use hyperlane_core::{ChainCommunicationError, FixedPointNumber};

/// Cosmos connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConf {
    /// The GRPC url to connect to
    grpc_url: String,
    /// The RPC url to connect to
    rpc_url: String,
    /// The chain ID
    chain_id: String,
    /// The human readable address prefix for the chains using bech32.
    bech32_prefix: String,
    /// Canoncial Assets Denom
    canonical_asset: String,
    /// The gas price set by the cosmos-sdk validator. Note that this represents the
    /// minimum price set by the validator.
    /// More details here: https://docs.cosmos.network/main/learn/beginner/gas-fees#antehandler
    gas_price: RawCosmosAmount,
    /// The number of bytes used to represent a contract address.
    /// Cosmos address lengths are sometimes less than 32 bytes, so this helps to serialize it in
    /// bech32 with the appropriate length.
    contract_address_bytes: usize,
}

/// Untyped cosmos amount
#[derive(serde::Serialize, serde::Deserialize, new, Clone, Debug)]
pub struct RawCosmosAmount {
    /// Coin denom (e.g. `untrn`)
    pub denom: String,
    /// Amount in the given denom
    pub amount: String,
}

/// Typed cosmos amount
#[derive(Clone, Debug)]
pub struct CosmosAmount {
    /// Coin denom (e.g. `untrn`)
    pub denom: String,
    /// Amount in the given denom
    pub amount: FixedPointNumber,
}

impl TryFrom<RawCosmosAmount> for CosmosAmount {
    type Error = ChainCommunicationError;
    fn try_from(raw: RawCosmosAmount) -> Result<Self, ChainCommunicationError> {
        Ok(Self {
            denom: raw.denom,
            amount: FixedPointNumber::from_str(&raw.amount)?,
        })
    }
}

/// An error type when parsing a connection configuration.
#[derive(thiserror::Error, Debug)]
pub enum ConnectionConfError {
    /// Missing `rpc_url` for connection configuration
    #[error("Missing `rpc_url` for connection configuration")]
    MissingConnectionRpcUrl,
    /// Missing `grpc_url` for connection configuration
    #[error("Missing `grpc_url` for connection configuration")]
    MissingConnectionGrpcUrl,
    /// Missing `chainId` for connection configuration
    #[error("Missing `chainId` for connection configuration")]
    MissingChainId,
    /// Missing `prefix` for connection configuration
    #[error("Missing `prefix` for connection configuration")]
    MissingPrefix,
    /// Invalid `url` for connection configuration
    #[error("Invalid `url` for connection configuration: `{0}` ({1})")]
    InvalidConnectionUrl(String, url::ParseError),
}

impl ConnectionConf {
    /// Get the GRPC url
    pub fn get_grpc_url(&self) -> String {
        self.grpc_url.clone()
    }

    /// Get the RPC url
    pub fn get_rpc_url(&self) -> String {
        self.rpc_url.clone()
    }

    /// Get the chain ID
    pub fn get_chain_id(&self) -> String {
        self.chain_id.clone()
    }

    /// Get the bech32 prefix
    pub fn get_bech32_prefix(&self) -> String {
        self.bech32_prefix.clone()
    }

    /// Get the asset
    pub fn get_canonical_asset(&self) -> String {
        self.canonical_asset.clone()
    }

    /// Get the minimum gas price
    pub fn get_minimum_gas_price(&self) -> RawCosmosAmount {
        self.gas_price.clone()
    }

    /// Get the number of bytes used to represent a contract address
    pub fn get_contract_address_bytes(&self) -> usize {
        self.contract_address_bytes
    }

    /// Create a new connection configuration
    pub fn new(
        grpc_url: String,
        rpc_url: String,
        chain_id: String,
        bech32_prefix: String,
        canonical_asset: String,
        minimum_gas_price: RawCosmosAmount,
        contract_address_bytes: usize,
    ) -> Self {
        Self {
            grpc_url,
            rpc_url,
            chain_id,
            bech32_prefix,
            canonical_asset,
            gas_price: minimum_gas_price,
            contract_address_bytes,
        }
    }
}

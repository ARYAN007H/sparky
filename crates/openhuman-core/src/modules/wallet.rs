//! Calling the `tinywallet` module.
//!
//! The host half of `ai.tinyhumans.tinywallet.Wallet`. Two ways to get a signed
//! transaction, and which one a chain uses is a property of that chain.
//!
//! # The confidential flow — preferred, and what BTC, EVM and Tron use
//!
//! [`sign_transaction_in_module`] sends the recovery phrase itself, once, and
//! the module derives, encodes, signs and assembles. No private key is ever
//! reassembled in this process.
//!
//! ```text
//!   host  ==SignTransaction{phrase, path, fields}==>  module   (confidential)
//!   host  <=={raw transaction, txid}================  module
//! ```
//!
//! The phrase only goes to a recipient tinybus has attested, and
//! [`attested_proxy`] additionally insists the attested digest is one
//! `registry.rs` pinned — see there for why both checks are worth having.
//!
//! # The split flow — still here, still correct for some hosts
//!
//! [`sign_transaction`] keeps the key in this process: the module returns
//! digests, this process signs them, the module reassembles.
//!
//! ```text
//!   host  --BuildUnsigned{fields, public key}-->  module
//!   host  <--[digests to sign]-------------------  module
//!   host    (signs locally, with a key the module never sees)
//!   host  --AttachSignature{fields, signatures}->  module
//!   host  <--{raw transaction, txid}-------------  module
//! ```
//!
//! It is not deprecated. It is the only option for a backend reached across a
//! transport, where the bus cannot say what is on the other end, and it is what
//! Solana still uses here — Solana hand-builds SPL messages that
//! `TransactionSpec::Solana` does not model, so there is nothing to send.
//!
//! # What the confidential flow does and does not buy
//!
//! A loaded module shares this address space and could read the phrase out of
//! process memory whichever flow is used, so neither is a hard isolation
//! boundary and neither is claimed as one.
//!
//! What changes is that this binary no longer performs derivation or signing at
//! all — the key exists only inside the module, for the duration of one call,
//! rather than being assembled here and held across two round trips. The bus
//! also refuses to carry the phrase to anything that is not an allowlisted,
//! hash-verified module, which the split flow could not express.
//!
//! # The fields are sent twice in the split flow, deliberately
//!
//! `AttachSignature` re-sends everything `BuildUnsigned` was given rather than a
//! handle to something the module remembered. That is what lets the module hold
//! no state between the calls — no store, no bound on it, no expiry for a host
//! that never comes back. Building is deterministic, so the module rebuilds the
//! transaction the digests were computed over. The confidential flow needs none
//! of this: it is one call.
//!
//! # Two signing schemes, and the difference matters
//!
//! A `Secp256k1Prehash` payload is **already hashed** and must be signed with a
//! prehash entry point; hashing it again produces a valid signature over the
//! wrong thing. An `Ed25519` payload is the whole message and must **not** be
//! pre-hashed, because ed25519 hashes internally. The module tags every payload
//! with which it is, and [`sign_payload`] dispatches on the tag rather than on
//! the chain — so a chain that changes scheme cannot silently sign wrongly.

use serde::de::DeserializeOwned;
use serde::Serialize;
use tinywallet_bus::names::methods;
use tinywallet_bus::wire::{
    DeriveMnemonicSeedRequest, DeriveMnemonicSeedResponse, DerivedAccount,
    EncodeErc20TransferRequest, EncodeErc20TransferResponse, ExportRequest, ExportedKey,
    GenerateMnemonicRequest, GenerateMnemonicResponse, Scheme, SecretMaterial, SignMessageRequest,
    SignRequest, Signature, SignedTransaction, TransactionSpec,
};
use zeroize::Zeroize;

use super::{host, ops, registry};
use crate::config::Config;

/// Registry id of the module these calls go to.
const MODULE_ID: &str = "tinywallet";

/// Why a wallet call did not produce a signed transaction.
///
/// Three variants because the wallet tools map them onto three different
/// user-facing outcomes: something the caller can correct, something it cannot,
/// and a capability that is not present on this host at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletCallError {
    /// The module is not loaded and cannot be: unsupported host, downloads off,
    /// disabled in config, or a load that already failed in this process.
    Unavailable(String),
    /// The request was rejected. The caller can act on this.
    InvalidInput(String),
    /// Building, signing, or assembly failed.
    Failed(String),
}

impl std::fmt::Display for WalletCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(message) | Self::InvalidInput(message) | Self::Failed(message) => {
                f.write_str(message)
            }
        }
    }
}

/// Derive, build, sign and assemble entirely inside the module.
///
/// The counterpart to [`sign_transaction`], and the one to prefer: the phrase
/// crosses the bus once and no key material is ever reassembled here.
///
/// # Errors
///
/// [`WalletCallError`]. `Unavailable` if the module is not an attested
/// recipient — see [`attested_proxy`], which refuses to send the phrase at all
/// in that case.
pub async fn sign_transaction_in_module(
    config: &Config,
    transaction: &TransactionSpec,
    secret: &SecretMaterial,
) -> Result<SignedTransaction, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] sign_transaction chain={:?} module={MODULE_ID} (confidential)",
        transaction.chain()
    );
    proxy
        .call_confidential(
            methods::SIGN_TRANSACTION,
            (SignRequest {
                secret: secret.clone(),
                transaction: transaction.clone(),
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Ask the module for an address and public key.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn derive_account(
    config: &Config,
    secret: &SecretMaterial,
) -> Result<DerivedAccount, WalletCallError> {
    log::debug!(
        "[modules:wallet] derive_account chain={:?} module={MODULE_ID} (confidential)",
        secret.chain
    );
    let mut request = secret.clone();
    let result = call_confidential(config, methods::DERIVE_ACCOUNT, &request).await;
    request.mnemonic.zeroize();
    result
}

/// Generate a recovery phrase inside the attested TinyWallet module.
///
/// # Errors
///
/// [`WalletCallError`]. The phrase is returned to the host for encryption and
/// custody, but generation and BIP-39 vocabulary remain module-owned.
pub async fn generate_mnemonic(
    config: &Config,
    word_count: u8,
) -> Result<GenerateMnemonicResponse, WalletCallError> {
    call_confidential(
        config,
        methods::GENERATE_MNEMONIC,
        &GenerateMnemonicRequest { word_count },
    )
    .await
}

/// Derive a standard BIP-39 seed inside the attested module.
///
/// The caller uses the seed only for OpenHuman's generic storage-key KDF.
///
/// # Errors
///
/// [`WalletCallError`]. Invalid or unsupported phrases are classified as
/// [`WalletCallError::InvalidInput`].
pub async fn derive_mnemonic_seed(
    config: &Config,
    mnemonic: String,
) -> Result<DeriveMnemonicSeedResponse, WalletCallError> {
    let mut request = DeriveMnemonicSeedRequest { mnemonic };
    let result = call_confidential(config, methods::DERIVE_MNEMONIC_SEED, &request).await;
    request.mnemonic.zeroize();
    result
}

/// Encode ERC-20 transfer calldata with the module-owned ABI implementation.
///
/// # Errors
///
/// Returns [`WalletCallError`] if the module is unavailable or rejects the
/// recipient or base-unit amount.
pub async fn encode_erc20_transfer(
    config: &Config,
    to_address: &str,
    amount_raw: &str,
) -> Result<String, WalletCallError> {
    let response: EncodeErc20TransferResponse = call(
        config,
        methods::ENCODE_ERC20_TRANSFER,
        &EncodeErc20TransferRequest {
            to_address: to_address.to_owned(),
            amount_raw: amount_raw.to_owned(),
        },
    )
    .await?;
    Ok(response.calldata)
}

/// Sign opaque bytes with the key derived from `secret`.
///
/// Blind: the module cannot check what the bytes mean, so
/// [`sign_transaction_in_module`] is the right call wherever the request can be
/// expressed as a `TransactionSpec`. This exists for the two encodings the wire
/// contract does not model — Solana SPL transfers and x402 payments — where the
/// alternative is not a verified signature but deriving the key in this process
/// and signing here, which is what these callers used to do.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn sign_message(
    config: &Config,
    secret: &SecretMaterial,
    message: &[u8],
    scheme: Scheme,
) -> Result<Signature, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] sign_message chain={:?} bytes={} module={MODULE_ID} (confidential)",
        secret.chain,
        message.len()
    );
    proxy
        .call_confidential(
            methods::SIGN_MESSAGE,
            (SignMessageRequest {
                secret: secret.clone(),
                message_hex: hex(message),
                scheme,
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Ask the module for the raw derived key.
///
/// A compatibility call for downstream hosts that must drive a signer locally.
/// OpenHuman's own wallet paths sign inside the module.
///
/// # Errors
///
/// [`WalletCallError`].
pub async fn export_key(
    config: &Config,
    secret: &SecretMaterial,
) -> Result<ExportedKey, WalletCallError> {
    let proxy = attested_proxy(config).await?;
    log::debug!(
        "[modules:wallet] export_key chain={:?} module={MODULE_ID} (confidential)",
        secret.chain
    );
    proxy
        .call_confidential(
            methods::EXPORT_KEY,
            (ExportRequest {
                secret: secret.clone(),
            },),
        )
        .await
        .map_err(|error| classify(&error))
}

/// Call a typed Wallet, Web3, or x402 member with one request argument.
/// Business rules stay in the module; the host only transports the request.
pub async fn call<Request, Reply>(
    config: &Config,
    member: &str,
    request: Request,
) -> Result<Reply, WalletCallError>
where
    Request: Serialize + Send,
    Reply: DeserializeOwned,
{
    let (runtime, record) = ready(config).await?;
    proxy(runtime, record)?
        .call::<Reply>(member, (request,))
        .await
        .map_err(|error| classify(&error))
}

/// Call a Wallet member carrying secret material over TinyBus's confidential
/// transport. The loaded module must be attested and match this build's pin.
pub async fn call_confidential<Request, Reply>(
    config: &Config,
    member: &str,
    request: &Request,
) -> Result<Reply, WalletCallError>
where
    Request: Serialize + Send,
    Reply: DeserializeOwned,
{
    attested_proxy(config)
        .await?
        .call_confidential::<Reply>(member, (request,))
        .await
        .map_err(|error| classify(&error))
}

/// A proxy that has proved it is the artifact this build pinned.
///
/// # Why check here when the broker already refuses
///
/// The broker will not route a confidential call to an unattested recipient, so
/// omitting this would still be safe against the case it covers. Two reasons to
/// check anyway.
///
/// The first is the error. A broker refusal arrives after the request has been
/// serialized, which means a frame containing the recovery phrase was built and
/// handed to the bus before anything said no. Checking first means the phrase is
/// never put into a buffer on a call that was always going to fail.
///
/// The second is that this compares the digest against **this build's own
/// table**, which the broker cannot do — it only knows the host vouched for
/// something. Here we can insist it vouched for one of the artifacts
/// `registry.rs` names. That closes the gap where a host is somehow induced to
/// attest a different artifact for this bus name.
///
/// Both are belt-and-braces over a check that already exists. That is the right
/// posture for the one code path that hands over a recovery phrase.
async fn attested_proxy(config: &Config) -> Result<tinybus::Proxy, WalletCallError> {
    let (runtime, record) = ready(config).await?;
    let proxy = proxy(runtime, record)?;

    let attestation = proxy
        .attestation()
        .await
        .map_err(|error| WalletCallError::Failed(error.to_string()))?
        .ok_or_else(|| {
            WalletCallError::Unavailable(
                "the wallet module is loaded but not an attested recipient, so it cannot be sent \
                 key material. This host is running a build whose module loader does not record \
                 an attestation for pinned releases; upgrade it rather than working around this."
                    .to_string(),
            )
        })?;

    if attestation.name.as_str() != record.bus_name {
        return Err(WalletCallError::Failed(format!(
            "the wallet module's attestation names '{}' rather than '{}'",
            attestation.name.as_str(),
            record.bus_name
        )));
    }

    if !digest_is_pinned(record, &attestation.sha256) {
        return Err(WalletCallError::Failed(
            "the loaded wallet module is not one of the artifacts this build pinned, so it will \
             not be sent key material"
                .to_string(),
        ));
    }

    Ok(proxy)
}

// Embedded library checksums verified from all 17 archives in the
// registry-pinned TinyWallet v0.19.2 release. Each library is trusted only
// while its archive checksum remains pinned in the supplied ModuleRecord.
const PINNED_WALLET_LIBRARIES: &[(&str, &str)] = &[
    // archlinux-arm64
    (
        "df51cab1066c4ea5b6964343a136769b53b61de392a8dcf56f9455b5370a72af",
        "792505e0b74acef2115ae93603774a6619d195d3bf52ead5782ef5f652750d83",
    ),
    // archlinux-x86_64
    (
        "e7b0717c82a530b2ca0f44c55938d66f19bb98b0f314032cf8f3336cf23840cc",
        "27cd08886778eaa4daf5727a15c8ff55d330c155263d33a5fe65ebc9d73e4bf2",
    ),
    // fedora-43-arm64
    (
        "c42776335a866fda439fc2d05ac70c6dd120fa0d56359d910bf5fe58e51b354b",
        "af3bdee4e19abdfe8a1230032056402dcc4f204e7567866ca012fc9083f8fbbd",
    ),
    // fedora-43-x86_64
    (
        "4a26baaabad59fa96c0a36b766d8352d8eb72a09d188f7313b136379db34cf73",
        "d606b2606d63f7ee1504c7cbc9af09f7c1d82c6c1e3cc4c12904c0aec5a02f0a",
    ),
    // fedora-44-arm64
    (
        "6ac6974fd601959eb8b8726d99abe64389445d53b71f263f5af58e05877ffbb5",
        "06137e611e156662fb1b9e0c1a41862921d8e138bb8e0bce1b0b8c184d0c543b",
    ),
    // fedora-44-x86_64
    (
        "7589757b00ce3e01e88f01c4d479a6ef2795bf3f361dab3972cde51939e6ef97",
        "da7ec4d2f54ad76f559fd6cf12c29d983d75008fb731b4cf5c2a306b2d0dfd1a",
    ),
    // macos-15-arm64
    (
        "4ab4d3b350c2a6684f06dc67f7eb8c54957a5b15390d7b37fb4c38a5a3d3fb6a",
        "cdca98fb9c921e8a9f5d877266ddb6c921e9b2a4aef83c5b26d6a26e1e155108",
    ),
    // macos-15-x86_64
    (
        "2bb7d195371382030c5894e14a57f5a792e3e41b36246bd754174e26b250bb94",
        "3629c3e513cab093933d9c10ad10f677eb0f8ea441ef75e527d8b2d2950c4878",
    ),
    // macos-26-arm64
    (
        "f5e36e43bf05321e0fa503f591e1f51471eb6ee06bc6dc39a2857a35aeecc24f",
        "a86f18de1988e8caf004c432b66d08ef9bc07cc3fc7370f6d80cbb9bc9761abe",
    ),
    // macos-26-x86_64
    (
        "37845264453470cdfee9fea998d4664e1ab948a566e1da2ea585fac53d13b9eb",
        "82862c07d959c4f1c3d403ae6a8201b6ffefc9eebcac6021a7b91c28584ac595",
    ),
    // ubuntu-22.04-arm64
    (
        "9311690732558c3347ff4a3bbe4a29a09cfd999ce53561080d58f3963d143a54",
        "bd72a70b9ddfd6a37d9c1bd422a7780fa3c019389cefb934f0629c5b84144d58",
    ),
    // ubuntu-22.04-x86_64
    (
        "016c5e6efc1bb63ce60572edc1ffd61699d3006601d5dc52e51604a91a225bf8",
        "3da718ca8e276f57846fe423e932b450175b3ab39630c3a14ee2cd169cbc5450",
    ),
    // ubuntu-24.04-arm64
    (
        "c68d3b8b2257377aed4d1b852a1eb28c26b4a7c34fb7eb7359720f098dc6216e",
        "61475f614731e554fd67f51d2a7823cc5cfcd84631651640e4648434a323a567",
    ),
    // ubuntu-24.04-x86_64
    (
        "0116da89b2031532b6eefa60f0d77335514c77dd7b5c49c6f556c180b3b24780",
        "0db559e23300432c7056cfe1fb9f8980a82faacacf9603131173967c80ba0d89",
    ),
    // windows-11-arm64
    (
        "977e2051dfadc29a7a66884198ac25fc6d4b2d375d45a174889158c6ca7e15f4",
        "ea230a805827a83be35a3b06b7f153934330386201525cfd3df11d52ee5f540b",
    ),
    // windows-2022-x86_64
    (
        "99a36e75a71212b22991564d10e18e0bbe3c1c0acae2d17bbe95df14eb6d2bdb",
        "313ad598eb2a30413683b812fb49b55c084df2fdaf109230c51b7da05770df26",
    ),
    // windows-2025-x86_64
    (
        "c16c1071651cd493dd56af5fdf63992c8667b8becaeac2ad0be4f32a7b6c493a",
        "1d0debad2b6446f43e98917979d2471b0fbc55d4ca65625b5efe6744013f9c83",
    ),
];

/// Whether the attested archive or library belongs to a currently pinned release.
///
/// The release loader attests the verified archive; local discovery attests the
/// library from modules.toml. Library checksums therefore require their paired
/// archive to remain in this record, so retiring a release also retires its
/// libraries. Check every platform because the attestation carries no host key.
fn digest_is_pinned(record: &super::ModuleRecord, sha256: &str) -> bool {
    record.assets.iter().any(|asset| {
        asset.sha256.eq_ignore_ascii_case(sha256)
            || PINNED_WALLET_LIBRARIES.iter().any(|(archive, library)| {
                asset.sha256.eq_ignore_ascii_case(archive) && library.eq_ignore_ascii_case(sha256)
            })
    })
}

/// Load the wallet module if it is not already serving.
///
/// Callers do not have to invoke this — the signing calls do — but a caller
/// that wraps its work in a deadline should, *outside* that deadline. A first
/// use may download and verify an artifact, and charging that against a
/// transaction timeout means the first transfer a user ever makes is the one
/// that fails.
///
/// # Errors
///
/// [`WalletCallError::Unavailable`].
pub async fn ensure_ready(config: &Config) -> Result<(), WalletCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(WalletCallError::Unavailable)
}

/// Ensure the module is serving and hand back what a call needs.
async fn ready(
    config: &Config,
) -> Result<(&'static host::ModuleRuntime, &'static super::ModuleRecord), WalletCallError> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(WalletCallError::Unavailable)?;
    let record = registry::find(MODULE_ID)
        .ok_or_else(|| WalletCallError::Unavailable(format!("unknown module '{MODULE_ID}'")))?;
    let runtime = host::runtime()
        .await
        .map_err(|_| WalletCallError::Unavailable("the module bus is not running".to_string()))?;
    Ok((runtime, record))
}

/// A proxy for the module's object.
fn proxy(
    runtime: &'static host::ModuleRuntime,
    record: &super::ModuleRecord,
) -> Result<tinybus::Proxy, WalletCallError> {
    runtime
        .proxy(record.bus_name, record.object_path)
        .map_err(|error| WalletCallError::Failed(error.to_string()))
}

/// Map a bus failure onto the shape a caller can act on.
///
/// The wire name is the contract; the message is for a human. An unrecognised
/// name is `Failed` rather than `InvalidInput`, because telling a caller its
/// input was wrong when it was not points at the wrong fix.
fn classify(error: &tinybus::Error) -> WalletCallError {
    let message = error.to_string();
    match error.wire_name() {
        "ai.tinyhumans.tinywallet.Error.InvalidInput" => WalletCallError::InvalidInput(message),
        "ai.tinyhumans.tinywallet.Error.UnsupportedChain" => WalletCallError::Unavailable(message),
        // The module is loaded but not answering: refused, faulted, or gone.
        name if name.contains("ModuleUnavailable") => WalletCallError::Unavailable(message),
        _ => WalletCallError::Failed(message),
    }
}

/// Lowercase hex, unprefixed — the form every field in the wire contract uses.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
#[path = "wallet_tests.rs"]
mod tests;

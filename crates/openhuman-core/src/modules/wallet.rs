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

use super::{ops, registry};
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
// registry-pinned TinyWallet v0.23.2 release. Each library is trusted only
// while its archive checksum remains pinned in the supplied ModuleRecord.
const PINNED_WALLET_LIBRARIES: &[(&str, &str)] = &[
    // archlinux-arm64
    (
        "a859a611dd4ffc9dedf2c843d33dcacc61d240eea42ed3a434795b82b52c4d13",
        "45d08584f367b1b30904933e6479dc57f678af03387ae787ba99aab01b4c8bf8",
    ),
    // archlinux-x86_64
    (
        "9fb038d83887402267713146fc37f61b5f275cd819e445caf2329bf4eb208d3d",
        "de0f92c61625f74f466216aeac9d9d25ac17d55ce4e3c7fe825be817881efb07",
    ),
    // fedora-43-arm64
    (
        "7b575ebab88691c89130919339cc83a4757b64bfe5f235494e18aeb181aaab8e",
        "43c3b7245a0dea84bcb7bb0b355fcc4eac7a554c59634239e66c7b2a42a6afad",
    ),
    // fedora-43-x86_64
    (
        "b02a10325ba0f1208e6e06dfabd79d878f78377c4ade5fc60617edf9f3c78a88",
        "ce2e2ac49db780bec10067e74d20afff662a7c246bc7bab9a77937443a6b4dac",
    ),
    // fedora-44-arm64
    (
        "40979de5b1a08eee59a370bef3d32a815b6a51aef3dae2958c2f53e568a390cc",
        "4500c3bfefaa29d31ef6c1f2d495c71b44288ab03bbf2207aad28dd40a868c1a",
    ),
    // fedora-44-x86_64
    (
        "690deeececa612327eb34ec81b4521446c221c90190466c40a316a7aab9a7141",
        "71763aa04627e807bedd5e5eb51ce5bbf9497d9a63ce78bbee12a98854d69910",
    ),
    // macos-15-arm64
    (
        "01bf9ab5be34862de7143c494354d72c06d5d81ad553fda36ce13f871d636469",
        "ae85ee115874ec7eb3e7fda49a3f6a008ee83a9a2db04cafe49a25e924e924c8",
    ),
    // macos-15-x86_64
    (
        "2d252e601f62d6ab76c61ed07c4282fa61316f51325fe8b45810199db26269f7",
        "6a3e3adb9481e13e255a5278df71776a38ca6488abffd8247856a3adbd5d85c3",
    ),
    // macos-26-arm64
    (
        "10a808ca83d6a10646146be738ebc958518116481d1599d7aaddc5c6fd468203",
        "ba6e4f6f0cfde71b62529aff79c8133f6848c58edcd8f7ccbd1fc41a0567e9bd",
    ),
    // macos-26-x86_64
    (
        "a49d02b756579f9945dae02a3fd383ba8fcf3039f12e3fa97e2cfb98f9cdcb99",
        "5d731ee132e128f51046d522bae8c3447c467e53f2ac58e41e6cdfbb325fbba8",
    ),
    // ubuntu-22.04-arm64
    (
        "8da89114a625fefff1822e2b54abbbc65f85788f8bdeae17732cf0713c51eddf",
        "32733979d6a035dbcadeea5ecd631c16230736275555468bc18142cb954e1714",
    ),
    // ubuntu-22.04-x86_64
    (
        "8bf8f7597e07fb7453c2c96392333519ea3ef0dc402447f35d2910ed3c9e4a25",
        "676323f630edb78e5c29ee0764cefee700956216b8423efff4f2a4f8eb8e8f1b",
    ),
    // ubuntu-24.04-arm64
    (
        "240aceaff4324312587d708c385806dbbd10e0fb71e782bd7c46a04e2606f481",
        "fc8d0547d66863ad6bb9746524af847391f9506a2bff16aa4f9f5d4737fbb7bb",
    ),
    // ubuntu-24.04-x86_64
    (
        "96c82af6e6c2349b353789f33e617e7dba5bb63c4e50c4de49e3b359557c09f9",
        "5a65288638a73aeb5ae975302c516a7b52d2b11896646f8c5465a359fb2c1788",
    ),
    // windows-11-arm64
    (
        "edad022df52b317defabfc465fac0d57ff63c9c406797354e94e35a5f0744849",
        "f5350d8b1c697a0a9b94874c76cc52c0de1518b7a312a4cb176121d61ff03e0c",
    ),
    // windows-2022-x86_64
    (
        "1f44dce0e0bb5cac84ec9a1f596b86cda28e82d4198157f2f1faa0075e38ceb2",
        "c78513a44346c3cce07506f8a027b241b430139b6ce79581b55a166ae65e0c4e",
    ),
    // windows-2025-x86_64
    (
        "d23eb02be466689258de016fdf84e1f8c971bb207abae6668168bb12f517f5bc",
        "28fe5c9a0d1d52fe84c92ec506719fab7f9b0d0c343a29fbfc7fac3904227327",
    ),
];

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
) -> Result<
    (
        &'static tinybus::module::ModuleRuntime,
        &'static super::ModuleRecord,
    ),
    WalletCallError,
> {
    ops::ensure_loaded(config, MODULE_ID)
        .await
        .map_err(WalletCallError::Unavailable)?;
    let record = registry::find(MODULE_ID)
        .ok_or_else(|| WalletCallError::Unavailable(format!("unknown module '{MODULE_ID}'")))?;
    let runtime = super::runtime::runtime()
        .await
        .map_err(|_| WalletCallError::Unavailable("the module bus is not running".to_string()))?;
    Ok((runtime, record))
}

/// A proxy for the module's object.
fn proxy(
    runtime: &'static tinybus::module::ModuleRuntime,
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

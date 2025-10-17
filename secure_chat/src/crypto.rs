// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Cryptography utilities.

use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use rsa::pkcs8::DecodePrivateKey;
use rsa::pkcs8::EncodePublicKey;
use rsa::pss::{BlindedSigningKey, Signature, VerifyingKey};
use rsa::signature::{Keypair, RandomizedSigner, Verifier};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;
use thiserror::Error;

use crate::constants::RSA_KEY_SIZE;

pub trait CryptoUtil {
    /// Encrypt data with the public key.
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaUtilError>;

    /// Decrypt data with the private key.
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaUtilError>;

    /// Sign data and return the signature.
    fn sign(&self, data: &[u8]) -> Signature;

    /// Verify a signature against the provided data.
    fn verify(&self, data: &[u8], sig: &Signature) -> bool;
}

#[derive(Debug, Error)]
pub enum RsaUtilError {
    /// An error from the [`rsa`] library occurred.
    #[error("RSA library error")]
    RsaError(#[from] rsa::errors::Error),

    /// An IO error occurred.
    #[error("IO error")]
    IoError(#[from] std::io::Error),

    /// An error occurred while decoding the PEM file.
    #[error("PEM decoding error")]
    Pkcs8Error(#[from] rsa::pkcs8::Error),

    /// An error occurred while encoding/decoding public key.
    #[error("Public key encoding error")]
    SpkiError(#[from] rsa::pkcs8::spki::Error),
}

// TODO: rename to a better name since this isn't just "config"
pub struct RsaUtil {
    priv_key: RsaPrivateKey,
    pub_key: RsaPublicKey,
    signing_key: BlindedSigningKey<Sha256>,
    verifying_key: VerifyingKey<Sha256>,
}

impl RsaUtil {
    /// Create a new RSA configuration with a randomly generated key pair.
    pub fn new() -> Result<Self, RsaUtilError> {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE)?;
        let pub_key = RsaPublicKey::from(&priv_key);
        let signing_key = BlindedSigningKey::<Sha256>::new(priv_key.clone());
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            pub_key,
            priv_key,
            signing_key,
            verifying_key,
        })
    }

    /// Get the base64url-encoded public key (DER format without headers).
    pub fn pubkey_base64url(&self) -> Result<String, RsaUtilError> {
        let der = self.pub_key.to_public_key_der()?;
        Ok(STANDARD_NO_PAD.encode(der))
    }

    /// Get the private key as PEM string for saving to file.
    pub fn privkey_pem(&self) -> Result<String, RsaUtilError> {
        use rsa::pkcs8::EncodePrivateKey;
        Ok(self.priv_key.to_pkcs8_pem(Default::default())?.to_string())
    }

    /// Get the public key as PEM string for saving to file.
    pub fn pubkey_pem(&self) -> Result<String, RsaUtilError> {
        Ok(self.pub_key.to_public_key_pem(Default::default())?)
    }

    /// Get a reference to the public key.
    pub fn pub_key(&self) -> &RsaPublicKey {
        &self.pub_key
    }

    /// Get a reference to the verifying key.
    pub fn verifying_key(&self) -> &VerifyingKey<Sha256> {
        &self.verifying_key
    }

    /// Get a reference to the signing key.
    pub fn signing_key(&self) -> &BlindedSigningKey<Sha256> {
        &self.signing_key
    }

    /// Create a new RSA configuration from a private key file.
    pub fn new_from_file<P: AsRef<Path>>(path: P) -> Result<Self, RsaUtilError> {
        let priv_key_pem = fs::read_to_string(&path)?;
        let priv_key = RsaPrivateKey::from_pkcs8_pem(&priv_key_pem)?;
        let pub_key = RsaPublicKey::from(&priv_key);
        let signing_key = BlindedSigningKey::<Sha256>::new(priv_key.clone());
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            pub_key,
            priv_key,
            signing_key,
            verifying_key,
        })
    }
}

impl CryptoUtil for RsaUtil {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaUtilError> {
        let mut rng = rand::thread_rng();
        let padding = Oaep::new::<Sha256>();
        let enc_data = self.pub_key.encrypt(&mut rng, padding, data)?;
        Ok(enc_data)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaUtilError> {
        let padding = Oaep::new::<Sha256>();
        let dec_data = self.priv_key.decrypt(padding, data)?;
        Ok(dec_data)
    }

    fn sign(&self, data: &[u8]) -> Signature {
        let mut rng = rand::thread_rng();
        self.signing_key.sign_with_rng(&mut rng, data)
    }

    fn verify(&self, data: &[u8], sig: &Signature) -> bool {
        self.verifying_key.verify(data, sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_rsa_util_new() {
        let rsa_util = RsaUtil::new();
        assert!(rsa_util.is_ok(), "Failed to create new RsaUtil");
    }

    #[test]
    fn test_encrypt_decrypt_simple() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"Hello, world! This is a test message for encryption.";

        // Encrypt the data
        let encrypted = rsa_util.encrypt(test_data).expect("Failed to encrypt data");
        assert_ne!(
            encrypted, test_data,
            "Encrypted data should be different from original"
        );

        // Decrypt the data
        let decrypted = rsa_util
            .decrypt(&encrypted)
            .expect("Failed to decrypt data");
        assert_eq!(decrypted, test_data, "Decrypted data should match original");
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"";

        let encrypted = rsa_util
            .encrypt(test_data)
            .expect("Failed to encrypt empty data");
        assert_ne!(
            encrypted, test_data,
            "Encrypted data should be different from original"
        );

        let decrypted = rsa_util
            .decrypt(&encrypted)
            .expect("Failed to decrypt empty data");
        assert_eq!(
            decrypted, test_data,
            "Decrypted empty data should match original"
        );
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        // RSA with OAEP padding has size limitations, so we test with data that fits
        // Specifically, for a 4096-bit key and SHA-256 OAEP, the max message size is 446 bytes
        let test_data = b"This is a medium-sized test message for RSA encryption testing. Lorem ipsum dolor sit amet. Qui corrupti expedita ab voluptatum voluptatibus sed maxime velit et praesentium quos rem quia neque ut inventore accusantium est omnis nihil. Qui voluptas delectus qui voluptas sapiente vel repudiandae dignissimos hic possimus tempora. Et harum natus qui enim alias qui numquam velit sit fugiat Quis et ratione rerum. Et odit eaque non obcaecati quis et";

        let encrypted = rsa_util
            .encrypt(test_data)
            .expect("Failed to encrypt large data");
        let decrypted = rsa_util
            .decrypt(&encrypted)
            .expect("Failed to decrypt large data");
        assert_eq!(
            decrypted, test_data,
            "Decrypted large data should match original"
        );
    }

    #[test]
    fn test_sign_verify_simple() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"This is a test message for signing and verification.";

        // Sign the data
        let signature = rsa_util.sign(test_data);

        // Verify the signature
        let is_valid = rsa_util.verify(test_data, &signature);
        assert!(is_valid, "Signature should be valid for the original data");
    }

    #[test]
    fn test_sign_verify_different_data() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let original_data = b"Original message";
        let modified_data = b"Modified message";

        // Sign the original data
        let signature = rsa_util.sign(original_data);

        // Verify with original data should succeed
        assert!(
            rsa_util.verify(original_data, &signature),
            "Signature should be valid for original data"
        );

        // Verify with modified data should fail
        assert!(
            !rsa_util.verify(modified_data, &signature),
            "Signature should be invalid for modified data"
        );
    }

    #[test]
    fn test_sign_verify_empty_data() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"";

        let signature = rsa_util.sign(test_data);
        let is_valid = rsa_util.verify(test_data, &signature);
        assert!(is_valid, "Signature should be valid for empty data");
    }

    #[test]
    fn test_multiple_encryptions_are_different() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"Test message for randomness check";

        let encrypted1 = rsa_util
            .encrypt(test_data)
            .expect("Failed first encryption");
        let encrypted2 = rsa_util
            .encrypt(test_data)
            .expect("Failed second encryption");

        // Due to randomness in OAEP padding, encryptions should be different
        assert_ne!(
            encrypted1, encrypted2,
            "Multiple encryptions of same data should be different"
        );

        // But both should decrypt to the same original data
        let decrypted1 = rsa_util
            .decrypt(&encrypted1)
            .expect("Failed first decryption");
        let decrypted2 = rsa_util
            .decrypt(&encrypted2)
            .expect("Failed second decryption");
        assert_eq!(decrypted1, test_data);
        assert_eq!(decrypted2, test_data);
    }

    #[test]
    fn test_signatures_are_deterministic_with_different_rng() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let test_data = b"Test message for signature consistency";

        let signature1 = rsa_util.sign(test_data);
        let signature2 = rsa_util.sign(test_data);

        // Both signatures should verify correctly
        assert!(rsa_util.verify(test_data, &signature1));
        assert!(rsa_util.verify(test_data, &signature2));
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let rsa_util = RsaUtil::new().expect("Failed to create RsaUtil");
        let invalid_data = vec![0u8; 512]; // Random bytes that aren't valid encrypted data

        let result = rsa_util.decrypt(&invalid_data);
        assert!(result.is_err(), "Decryption of invalid data should fail");
    }

    #[test]
    fn test_cross_instance_verification_fails() {
        let rsa_util1 = RsaUtil::new().expect("Failed to create first RsaUtil");
        let rsa_util2 = RsaUtil::new().expect("Failed to create second RsaUtil");
        let test_data = b"Test message for cross-instance verification";

        // Sign with first instance
        let signature = rsa_util1.sign(test_data);

        // Verify with second instance should fail (different keys)
        let is_valid = rsa_util2.verify(test_data, &signature);
        assert!(
            !is_valid,
            "Signature from one instance should not verify with another instance's key"
        );
    }

    #[test]
    fn test_new_from_file_with_valid_key() {
        // Create a temporary RSA private key file
        let rsa_util_original = RsaUtil::new().expect("Failed to create original RsaUtil");

        // Export the private key in PKCS#8 PEM format
        use rsa::pkcs8::EncodePrivateKey;
        let private_key_pem = rsa_util_original
            .priv_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("Failed to encode private key");

        // Write to temporary file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(private_key_pem.as_bytes())
            .expect("Failed to write key to temp file");
        temp_file.flush().expect("Failed to flush temp file");

        // Create RsaUtil from file
        let rsa_util_from_file =
            RsaUtil::new_from_file(temp_file.path()).expect("Failed to create RsaUtil from file");

        // Test that the loaded key works
        let test_data = b"Test message for file-loaded key";
        let encrypted = rsa_util_from_file
            .encrypt(test_data)
            .expect("Failed to encrypt with file-loaded key");
        let decrypted = rsa_util_from_file
            .decrypt(&encrypted)
            .expect("Failed to decrypt with file-loaded key");
        assert_eq!(decrypted, test_data);

        let signature = rsa_util_from_file.sign(test_data);
        assert!(rsa_util_from_file.verify(test_data, &signature));
    }

    #[test]
    fn test_new_from_file_with_nonexistent_file() {
        let result = RsaUtil::new_from_file("/nonexistent/path/key.pem");
        assert!(
            result.is_err(),
            "Creating RsaUtil from nonexistent file should fail"
        );

        if let Err(e) = result {
            match e {
                RsaUtilError::IoError(_) => (),
                _ => panic!("Expected IoError, got {:?}", e),
            }
        }
    }

    #[test]
    fn test_new_from_file_with_invalid_key_content() {
        // Create a temporary file with invalid PEM content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"This is not a valid PEM key file")
            .expect("Failed to write invalid content");
        temp_file.flush().expect("Failed to flush temp file");

        let result = RsaUtil::new_from_file(temp_file.path());
        assert!(
            result.is_err(),
            "Creating RsaUtil from invalid key file should fail"
        );

        if let Err(e) = result {
            match e {
                RsaUtilError::Pkcs8Error(_) => (),
                _ => panic!("Expected Pkcs8Error, got {:?}", e),
            }
        }
    }

    #[test]
    fn test_error_types_display() {
        // Test that error types implement Display properly
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let rsa_util_error = RsaUtilError::IoError(io_error);
        let error_string = format!("{}", rsa_util_error);
        assert!(error_string.contains("IO error"));
    }

    #[test]
    fn test_concurrent_operations() {
        use std::sync::Arc;
        use std::thread;

        let rsa_util = Arc::new(RsaUtil::new().expect("Failed to create RsaUtil"));
        let mut handles = vec![];

        // Test concurrent signing operations
        for i in 0..5 {
            let rsa_util_clone = Arc::clone(&rsa_util);
            let handle = thread::spawn(move || {
                let test_data = format!("Test message {}", i).into_bytes();
                let signature = rsa_util_clone.sign(&test_data);
                rsa_util_clone.verify(&test_data, &signature)
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().expect("Thread panicked");
            assert!(result, "Concurrent signature verification should succeed");
        }
    }
}

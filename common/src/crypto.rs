use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a secure random token
pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Hash a token for secure storage
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a token against its hash
pub fn verify_token(token: &str, hash: &str) -> bool {
    hash_token(token) == hash
}

/// Generate a client ID
pub fn generate_client_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token1 = generate_token();
        let token2 = generate_token();

        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 64); // 32 bytes * 2 (hex)
    }

    #[test]
    fn test_token_hashing() {
        let token = "test-token";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        assert_eq!(hash1, hash2);
        assert!(verify_token(token, &hash1));
        assert!(!verify_token("wrong-token", &hash1));
    }

    #[test]
    fn test_client_id_generation() {
        let id1 = generate_client_id();
        let id2 = generate_client_id();

        assert_ne!(id1, id2);
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
    }
}

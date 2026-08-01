use x25519_dalek::{EphemeralSecret, PublicKey};

pub fn generate_auth_token() -> (EphemeralSecret, PublicKey) {
    let secret_key = EphemeralSecret::random();
    let public_key = PublicKey::from(&secret_key);

    (secret_key, public_key)
}

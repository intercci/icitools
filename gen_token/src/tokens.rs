use anyhow::Result;
use dotenv::dotenv;
use base64::prelude::*;
use pasetors::claims::{Claims, ClaimsValidationRules};
use pasetors::keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, Generate};
use pasetors::token::UntrustedToken;
use pasetors::{Public, public, version4::V4};
use std::env;
use std::time::Duration;

const SECONDS: u64 = 8 * 3600; // 8-hour life

#[derive(Debug, Clone)]
pub struct Keys {
    pub keys: AsymmetricKeyPair<V4>,
}

impl Keys {
    /// Create a new Keys instance with keys from environment variables
    ///
    /// Environment variables:
    /// - LAMBDA_TASK_ROOT: if exists, it's deployed on remote
    /// - PUB_KEY: the public key string
    /// - PRV_KEY: the private key string
    pub fn new() -> Result<Self> {
        dotenv().ok();
        let pb = env::var("PUB_KEY")?;
        let pv = env::var("PRV_KEY")?;
        let bbs = BASE64_STANDARD.decode(pb).unwrap();
        let vbs = BASE64_STANDARD.decode(pv).unwrap();
        let k = Keys {
            keys: AsymmetricKeyPair::<V4> {
                public: AsymmetricPublicKey::<V4>::from(bbs.as_ref()).unwrap(),
                secret: AsymmetricSecretKey::<V4>::from(vbs.as_ref()).unwrap(),
            },
        };
        Ok(k)
    }

    /// Create a new Keys instance with a pair of generated random keys.
    ///
    pub fn new_keys() -> Self {
        Keys {
            keys: AsymmetricKeyPair::<V4>::generate().unwrap(),
        }
    }

    /// Return the base64-encoded string of the current public key.
    ///
    pub fn public_key_string(&self) -> String {
        BASE64_STANDARD.encode(self.keys.public.as_bytes())
    }

    /// Return the base64-encoded string of the current private key.
    ///
    pub fn private_key_string(&self) -> String {
        BASE64_STANDARD.encode(self.keys.secret.as_bytes())
    }

    /// Make a PASETO token with sub for testing purpose.
    ///
    pub fn gen_token(&self, sub: &str, aud: &str, role: &str) -> Result<String> {
        // let mut claims = Claims::new()?;
        let duration = Duration::new(SECONDS, 0);
        let mut claims = Claims::new_expires_in(&duration)?;
        claims.subject(sub)?;
        claims.audience(aud)?;
        claims.add_additional("role", role)?;
        let t = public::sign(&self.keys.secret, &claims, None, None)?;
        Ok(t)
    }

    /// Verify a PASETO token and return the Claims.
    ///
    /// Errors:
    /// - Invalid or expired token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let tokens = if token.starts_with("Bearer ") {
            &token[7..]
        } else {
            token
        };
        // println!("Got token: {}", tokens);
        let untrusted_token = UntrustedToken::<Public, V4>::try_from(tokens)?;
        let validation_rules = ClaimsValidationRules::new();
        let r = public::verify(
            &self.keys.public,
            &untrusted_token,
            &validation_rules,
            None,
            None,
        )?;
        Ok(r.payload_claims().unwrap().to_owned())
    }
}

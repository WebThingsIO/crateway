use crate::{
    db::{CreateJwt, Db, GetJwtPublicKeyByKeyId},
    macros::call,
};
use anyhow::{anyhow, Context, Error};
use chrono::{Duration, Utc};
use hex;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use openssl::{
    ec::{EcGroup, EcKey},
    nid::Nid,
    pkey::PKey,
};
use rocket::{
    http::Status,
    request::{self, FromRequest, Outcome, Request},
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Role {
    UserToken,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    role: Role,
    user: i64,
    exp: usize,
}

pub struct JSONWebToken(pub TokenData<Claims>);

impl Deref for JSONWebToken {
    type Target = TokenData<Claims>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl JSONWebToken {
    pub fn user_id(&self) -> i64 {
        self.claims.user
    }
}

async fn decode_token(token: &str) -> Result<TokenData<Claims>, Error> {
    let kid = jsonwebtoken::decode_header(token)?
        .kid
        .ok_or_else(|| anyhow!("Failed to obtain kid"))?;
    let pub_key = call!(Db.GetJwtPublicKeyByKeyId(kid))?;
    jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_ec_pem(&hex::decode(pub_key)?)?,
        &Validation::new(Algorithm::ES256),
    )
    .context("Decode jwt")
}

fn extract_jwt_query(request: &Request<'_>) -> Option<String> {
    if let Some(Ok(jwt)) = request.query_value("jwt") {
        Some(jwt)
    } else {
        None
    }
}

fn extract_jwt_header(request: &Request<'_>) -> Option<String> {
    let keys: Vec<_> = request.headers().get("Authorization").collect();
    if keys.len() != 1 {
        return None;
    }
    let mut jwt = keys[0].split_whitespace();
    if jwt.next() != Some("Bearer") {
        return None;
    }
    jwt.next().map(|token| token.to_string())
}

fn extract_jwt(request: &Request<'_>) -> Option<String> {
    if let Some(token) = extract_jwt_header(request) {
        return Some(token);
    }
    if let Some(token) = extract_jwt_query(request) {
        return Some(token);
    }
    None
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for JSONWebToken {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match extract_jwt(request) {
            Some(token) => match decode_token(&token).await {
                Ok(jwt) => Outcome::Success(JSONWebToken(jwt)),
                Err(err) => {
                    error!("Authorization failed: {:?}", err);
                    Outcome::Failure((Status::Unauthorized, "Authorization invalid"))
                }
            },
            _ => Outcome::Failure((Status::Unauthorized, "Authorization missing")),
        }
    }
}

pub async fn issue_token(user_id: i64) -> Result<String, Error> {
    let (pub_key, priv_key) = generate_key_pair()?;
    let claims = Claims {
        user: user_id,
        role: Role::UserToken,
        exp: Utc::now()
            .checked_add_signed(Duration::days(365))
            .ok_or_else(|| anyhow!("Failed to calculate expiration date"))?
            .timestamp() as usize,
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(Uuid::new_v4().to_string());
    let token = jsonwebtoken::encode(&header, &claims, &EncodingKey::from_ec_pem(&priv_key)?)
        .context("Failed to encode JWT")?;
    let kid = header.kid.ok_or_else(|| anyhow!("Kid missing"))?;
    call!(Db.CreateJwt(kid, user_id, hex::encode(pub_key)))?;
    Ok(token)
}

fn generate_key_pair() -> Result<(Vec<u8>, Vec<u8>), Error> {
    let curve = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let ec = EcKey::generate(&curve)?;
    let pkey = PKey::from_ec_key(ec)?;

    let pub_key: Vec<u8> = pkey.public_key_to_pem()?;
    let priv_key: Vec<u8> = pkey.private_key_to_pem_pkcs8()?;

    Ok((pub_key, priv_key))
}

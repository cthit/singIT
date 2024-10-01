use std::sync::Arc;

use actix_session::Session;
use actix_utils::future::{ready, Ready};
use actix_web::{
    error, get,
    web::{self, Json, Redirect},
    http::StatusCode,
    FromRequest, HttpRequest, Responder,
};
use eyre::eyre;
use gamma_rust_client::{
    config::GammaConfig,
    oauth::{gamma_init_auth, GammaAccessToken, GammaOpenIDUser, GammaState},
};
use serde::{Deserialize, Serialize};
use singit_lib::UserInfo;

const ACCESS_TOKEN_SESSION_KEY: &str = "access_token";
const GAMMA_AUTH_STATE_KEY: &str = "GAMMA_AUTH_STATE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub access_token: GammaAccessToken,
    pub info: GammaOpenIDUser,
}

#[derive(Deserialize)]
struct RedirectParams {
    state: String,
    code: String,
}

#[get("/me")]
pub async fn user_info(user: Option<User>) -> (Json<Option<UserInfo>>, StatusCode) {
    let user = user.map(|user| UserInfo {
        cid: user.info.cid,
        nick: user.info.nick,
    });

    let status = if user.is_some() {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };

    (Json(user), status)
}

impl FromRequest for User {
    type Error = actix_web::error::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let mut f = || {
            let session = Session::from_request(req, payload)
                .into_inner()
                // TODO: is this right?
                .map_err(error::ErrorInternalServerError)?;

            let user: User = session
                .get(ACCESS_TOKEN_SESSION_KEY)
                .ok()
                .flatten()
                .ok_or(eyre!("Not logged in"))
                .map_err(error::ErrorUnauthorized)?;

            Ok(user)
        };

        ready(f())
    }
}

#[get("/logout")]
pub async fn logout(session: Session) -> impl Responder {
    session.clear();

    Redirect::to("/").temporary()
}

#[get("/login/gamma")]
pub async fn login_with_gamma(
    gamma_config: web::Data<Arc<GammaConfig>>,
    session: Session,
) -> impl Responder {
    let gamma_auth = gamma_init_auth(&gamma_config).expect("Failed to do gamma auth");

    session
        .insert(
            GAMMA_AUTH_STATE_KEY.to_string(),
            gamma_auth.state.get_state(),
        )
        .expect("Failed to set state cookie");

    Redirect::to(gamma_auth.redirect_to).temporary()
}

#[get("/login/gamma/redirect")]
pub async fn gamma_redirect(
    params: web::Query<RedirectParams>,
    gamma_config: web::Data<Arc<GammaConfig>>,
    //opt: web::Data<Arc<Opt>>,
    session: Session,
) -> impl Responder {
    let state: String = session
        .get(GAMMA_AUTH_STATE_KEY)
        .expect("Failed to read session store")
        .expect("Failed to deserialize gamma auth state key");
    let state = GammaState::get_state_str(state);

    let access_token = state
        .gamma_callback_params(&gamma_config, &params.state, params.code.clone())
        .await
        .expect("Failed to verify gamma redirect");

    let user = access_token
        .get_current_user(&gamma_config)
        .await
        .expect("Failed to get gamma user info");

    let user = User {
        access_token,
        info: user,
    };

    session
        .insert(ACCESS_TOKEN_SESSION_KEY, user)
        .expect("Failed to insert auth token in session");

    Redirect::to("/").temporary()
}
